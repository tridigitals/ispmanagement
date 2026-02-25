<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import {
    api,
    type CustomerLocation,
    type CustomerSubscriptionView,
    type IspPackage,
  } from '$lib/api/client';
  import { appSettings } from '$lib/stores/settings';
  import { toast } from '$lib/stores/toast';

  let loading = $state(true);
  let locations = $state<CustomerLocation[]>([]);
  let packages = $state<IspPackage[]>([]);
  let subscriptions = $state<CustomerSubscriptionView[]>([]);
  let selectedLocationId = $state('');
  let selectedCycle = $state<'monthly' | 'yearly'>('monthly');
  let purchasingPackageId = $state<string | null>(null);
  let loadError = $state('');
  let showCheckoutModal = $state(false);
  let checkoutCandidate = $state<IspPackage | null>(null);
  let showAddLocationModal = $state(false);
  let creatingLocation = $state(false);
  let newLocationLabel = $state('');
  let newLocationAddress = $state('');
  let newLocationCity = $state('');
  let newLocationState = $state('');
  let newLocationPostalCode = $state('');
  let newLocationCountry = $state('ID');
  let newLocationNotes = $state('');

  onMount(() => {
    void loadData();
  });

  async function loadData() {
    loading = true;
    loadError = '';
    try {
      const [myLocations, myPackages, mySubscriptions] = await Promise.all([
        api.customers.portal.myLocations(),
        api.customers.portal.myPackages(),
        api.customers.portal.mySubscriptions({ page: 1, per_page: 100 }),
      ]);

      locations = myLocations || [];
      packages = (myPackages || []).filter((p) => p.is_active);
      subscriptions = mySubscriptions?.data || [];

      if (!selectedLocationId && locations.length > 0) {
        selectedLocationId = locations[0].id;
      }
    } catch (e: any) {
      loadError = e?.message || String(e);
      toast.error(
        get(t)('dashboard.packages.toasts.load_failed') || 'Failed to load package catalog',
      );
    } finally {
      loading = false;
    }
  }

  const currentSubscription = $derived.by(() => {
    if (!selectedLocationId) return null;
    return (
      subscriptions.find((s) => s.location_id === selectedLocationId && s.status === 'active') ||
      null
    );
  });
  const selectedLocation = $derived.by(
    () => locations.find((location) => location.id === selectedLocationId) || null,
  );

  function hasYearlyPrice(pkg: IspPackage) {
    return Number(pkg.price_yearly || 0) > 0;
  }

  function getPrice(pkg: IspPackage) {
    if (selectedCycle === 'yearly' && hasYearlyPrice(pkg)) return Number(pkg.price_yearly || 0);
    return Number(pkg.price_monthly || 0);
  }

  function formatCurrency(amount: number) {
    const currency = ($appSettings as any)?.currency_code || 'IDR';
    const locale = ($appSettings as any)?.default_locale || 'id-ID';
    try {
      return new Intl.NumberFormat(locale, { style: 'currency', currency }).format(amount || 0);
    } catch {
      return `${currency} ${Number(amount || 0).toLocaleString(locale)}`;
    }
  }

  function packageActionLabel(pkg: IspPackage) {
    if (currentSubscription?.package_id === pkg.id) {
      return get(t)('dashboard.packages.actions.current') || 'Current package';
    }
    if (currentSubscription) {
      return get(t)('dashboard.packages.actions.switch') || 'Switch package';
    }
    return get(t)('dashboard.packages.actions.buy') || 'Buy package';
  }

  function checkoutEligibilityError(pkg: IspPackage, cycle: 'monthly' | 'yearly'): string | null {
    if (!selectedLocationId) {
      return get(t)('dashboard.packages.toasts.select_location') || 'Select a location first';
    }

    if (cycle === 'yearly' && !hasYearlyPrice(pkg)) {
      return (
        get(t)('dashboard.packages.toasts.yearly_unavailable') ||
        'Yearly billing is not available for this package'
      );
    }

    if (
      currentSubscription?.package_id === pkg.id &&
      currentSubscription?.billing_cycle === cycle
    ) {
      return get(t)('dashboard.packages.toasts.already_active') || 'This package is already active';
    }

    return null;
  }

  function requestCheckout(pkg: IspPackage) {
    const eligibilityError = checkoutEligibilityError(pkg, 'monthly');
    if (eligibilityError) {
      toast.info(eligibilityError);
      return;
    }
    selectedCycle = 'monthly';
    checkoutCandidate = pkg;
    showCheckoutModal = true;
  }

  async function confirmCheckout() {
    const pkg = checkoutCandidate;
    if (!pkg) return;
    const eligibilityError = checkoutEligibilityError(pkg, selectedCycle);
    if (eligibilityError) {
      toast.info(eligibilityError);
      return;
    }

    purchasingPackageId = pkg.id;
    try {
      const res = await api.customers.portal.checkout({
        location_id: selectedLocationId,
        package_id: pkg.id,
        billing_cycle: selectedCycle,
      });
      showCheckoutModal = false;
      checkoutCandidate = null;

      const invoiceNumber = res.invoice?.invoice_number || res.invoice?.id;
      if (res.invoice?.status === 'paid') {
        toast.info(
          `${get(t)('dashboard.packages.toasts.already_active') || 'This package is already active'} (${invoiceNumber})`,
        );
        goto('/dashboard/invoices');
        return;
      }

      toast.success(
        `${get(t)('dashboard.packages.toasts.checkout_success') || 'Invoice ready for payment.'} (${invoiceNumber})`,
      );
      await loadData();
      goto(`/pay/${res.invoice.id}`);
    } catch (e: any) {
      toast.error(
        e?.message || get(t)('dashboard.packages.toasts.checkout_failed') || 'Checkout failed',
      );
    } finally {
      purchasingPackageId = null;
    }
  }

  function openAddLocationModal() {
    newLocationLabel = '';
    newLocationAddress = '';
    newLocationCity = '';
    newLocationState = '';
    newLocationPostalCode = '';
    newLocationCountry = 'ID';
    newLocationNotes = '';
    showAddLocationModal = true;
  }

  async function saveMyLocation() {
    if (creatingLocation || !newLocationLabel.trim()) return;
    creatingLocation = true;
    try {
      await api.customers.portal.createMyLocation({
        label: newLocationLabel.trim(),
        address_line1: newLocationAddress.trim() || null,
        city: newLocationCity.trim() || null,
        state: newLocationState.trim() || null,
        postal_code: newLocationPostalCode.trim() || null,
        country: newLocationCountry.trim() || null,
        notes: newLocationNotes.trim() || null,
      });
      toast.success($t('common.saved') || 'Saved');
      showAddLocationModal = false;
      await loadData();
      if (locations.length > 0) {
        selectedLocationId = locations[0].id;
      }
    } catch (e: any) {
      toast.error(e?.message || 'Failed to create location');
    } finally {
      creatingLocation = false;
    }
  }
</script>

<div class="packages-page fade-in">
  <header class="page-header">
    <div>
      <h1>{$t('dashboard.packages.title') || 'Internet Packages'}</h1>
      <p class="subtitle">
        {$t('dashboard.packages.subtitle') ||
          'Choose your package and generate an invoice instantly for payment.'}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={() => goto('/dashboard/invoices')}>
        <Icon name="file-text" size={16} />
        {$t('admin.invoices.title') || 'Invoices'}
      </button>
      <button class="btn btn-secondary" onclick={loadData} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </header>

  {#if loadError}
    <div class="alert alert-error">{loadError}</div>
  {/if}

  <section class="controls card">
    <div class="field">
      <label for="location">{$t('dashboard.packages.fields.location') || 'Location'}</label>
      <select
        id="location"
        bind:value={selectedLocationId}
        disabled={loading || locations.length === 0}
      >
        {#if locations.length === 0}
          <option value=""
            >{$t('dashboard.packages.empty.no_locations') || 'No locations available'}</option
          >
        {/if}
        {#each locations as location (location.id)}
          <option value={location.id}>{location.label}</option>
        {/each}
      </select>
    </div>

    <div class="current-status">
      <div class="status-label">
        {$t('dashboard.packages.current.title') || 'Current subscription'}
      </div>
      {#if currentSubscription}
        <div class="status-value">
          {currentSubscription.package_name || currentSubscription.package_id}
        </div>
        <div class="status-sub">
          {currentSubscription.billing_cycle} · {formatCurrency(currentSubscription.price)}
        </div>
      {:else}
        <div class="status-empty">
          {$t('dashboard.packages.current.none') || 'No active package on this location'}
        </div>
      {/if}
    </div>
  </section>

  {#if !loading && locations.length === 0}
    <div class="empty-block card">
      <Icon name="map-pin" size={20} />
      <div>
        <h3>{$t('dashboard.packages.empty.no_locations') || 'No locations available'}</h3>
        <p>
          {$t('dashboard.packages.empty.no_locations_hint') ||
            'Ask admin to link your account with a customer location first.'}
        </p>
        <div class="empty-actions">
          <button class="btn btn-primary" onclick={openAddLocationModal}>
            <Icon name="plus" size={16} />
            {$t('dashboard.packages.actions.add_location') || 'Add location'}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <section class="package-grid">
    {#if loading}
      {#each Array(3) as _, i}
        <article class="package-card card skeleton" aria-hidden="true"></article>
      {/each}
    {:else if packages.length === 0}
      <div class="empty-block card">
        <Icon name="package" size={20} />
        <div>
          <h3>{$t('dashboard.packages.empty.no_packages') || 'No active packages yet'}</h3>
          <p>
            {$t('dashboard.packages.empty.no_packages_hint') ||
              'Your admin has not published package catalog yet.'}
          </p>
        </div>
      </div>
    {:else}
      {#each packages as pkg (pkg.id)}
        <article
          class="package-card card {currentSubscription?.package_id === pkg.id ? 'active' : ''}"
        >
          <div class="package-top">
            <h3>{pkg.name}</h3>
            {#if currentSubscription?.package_id === pkg.id}
              <span class="badge">{$t('dashboard.packages.badges.active') || 'Active'}</span>
            {/if}
          </div>

          {#if pkg.description}
            <p class="description">{pkg.description}</p>
          {/if}

          <div class="price">{formatCurrency(Number(pkg.price_monthly || 0))}</div>
          <div class="price-sub">
            {$t('dashboard.packages.cycles.monthly') || 'Monthly'}
            {#if hasYearlyPrice(pkg)}
              · {$t('dashboard.packages.cycles.yearly') || 'Yearly'} {$t('common.available') || 'available'}
            {/if}
          </div>

          {#if pkg.features?.length}
            <ul class="features">
              {#each pkg.features.slice(0, 4) as feature}
                <li>
                  <Icon name="check" size={14} />
                  <span>{feature}</span>
                </li>
              {/each}
            </ul>
          {/if}

          <button
            class="btn btn-primary"
            disabled={!selectedLocationId ||
              purchasingPackageId === pkg.id}
            onclick={() => requestCheckout(pkg)}
          >
            {#if purchasingPackageId === pkg.id}
              <Icon name="refresh-cw" size={16} />
              {$t('common.loading') || 'Loading...'}
            {:else}
              <Icon name="credit-card" size={16} />
              {packageActionLabel(pkg)}
            {/if}
          </button>
        </article>
      {/each}
    {/if}
  </section>
</div>

<Modal
  show={showCheckoutModal}
  title={$t('dashboard.packages.actions.buy') || 'Buy package'}
  onclose={() => {
    showCheckoutModal = false;
    checkoutCandidate = null;
  }}
>
  {#if checkoutCandidate}
    <div class="checkout-modal">
      <div class="checkout-summary">
        <div>
          <small>Package</small>
          <strong>{checkoutCandidate.name}</strong>
        </div>
        <div>
          <small>Location</small>
          <strong>{selectedLocation?.label || '-'}</strong>
        </div>
        <div>
          <small>Billing cycle</small>
          <strong>{selectedCycle === 'yearly' ? 'Yearly' : 'Monthly'}</strong>
        </div>
        <div>
          <small>Total invoice</small>
          <strong>{formatCurrency(getPrice(checkoutCandidate))}</strong>
        </div>
      </div>
      <div class="cycle-pills">
        <button
          class="cycle-pill {selectedCycle === 'monthly' ? 'active' : ''}"
          type="button"
          onclick={() => (selectedCycle = 'monthly')}
          disabled={!!purchasingPackageId}
        >
          {$t('dashboard.packages.cycles.monthly') || 'Monthly'}
        </button>
        <button
          class="cycle-pill {selectedCycle === 'yearly' ? 'active' : ''}"
          type="button"
          onclick={() => (selectedCycle = 'yearly')}
          disabled={!!purchasingPackageId || !hasYearlyPrice(checkoutCandidate)}
        >
          {$t('dashboard.packages.cycles.yearly') || 'Yearly'}
        </button>
      </div>
      <p class="checkout-note">
        Checkout akan membuat invoice otomatis untuk periode berjalan. Jika invoice periode ini sudah
        ada, sistem akan menggunakan invoice yang sama.
      </p>
      <div class="checkout-actions">
        <button
          class="btn btn-secondary"
          onclick={() => {
            showCheckoutModal = false;
            checkoutCandidate = null;
          }}
          disabled={!!purchasingPackageId}
        >
          {$t('common.cancel') || 'Cancel'}
        </button>
        <button
          class="btn btn-primary"
          onclick={confirmCheckout}
          disabled={purchasingPackageId === checkoutCandidate?.id}
        >
          {#if purchasingPackageId === checkoutCandidate.id}
            <Icon name="refresh-cw" size={16} />
            {$t('common.loading') || 'Loading...'}
          {:else}
            <Icon name="credit-card" size={16} />
            {$t('dashboard.packages.actions.buy') || 'Buy package'}
          {/if}
        </button>
      </div>
    </div>
  {/if}
</Modal>

<Modal
  show={showAddLocationModal}
  title={$t('dashboard.packages.actions.add_location') || 'Add location'}
  onclose={() => {
    if (!creatingLocation) showAddLocationModal = false;
  }}
>
  <div class="location-form">
    <label class="form-field">
      <span>Label</span>
      <input class="input" bind:value={newLocationLabel} placeholder="Home / Office" />
    </label>
    <label class="form-field">
      <span>Address</span>
      <input class="input" bind:value={newLocationAddress} placeholder="Street address" />
    </label>
    <div class="location-grid-2">
      <label class="form-field">
        <span>City</span>
        <input class="input" bind:value={newLocationCity} />
      </label>
      <label class="form-field">
        <span>State</span>
        <input class="input" bind:value={newLocationState} />
      </label>
      <label class="form-field">
        <span>Postal code</span>
        <input class="input" bind:value={newLocationPostalCode} />
      </label>
      <label class="form-field">
        <span>Country</span>
        <input class="input" bind:value={newLocationCountry} />
      </label>
    </div>
    <label class="form-field">
      <span>Notes</span>
      <textarea class="input textarea" bind:value={newLocationNotes} rows="3"></textarea>
    </label>
    <div class="checkout-actions">
      <button
        class="btn btn-secondary"
        onclick={() => (showAddLocationModal = false)}
        disabled={creatingLocation}
      >
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn btn-primary" onclick={saveMyLocation} disabled={creatingLocation || !newLocationLabel.trim()}>
        {#if creatingLocation}
          <Icon name="refresh-cw" size={16} />
          {$t('common.loading') || 'Loading...'}
        {:else}
          <Icon name="save" size={16} />
          {$t('common.save') || 'Save'}
        {/if}
      </button>
    </div>
  </div>
</Modal>

<style>
  .packages-page {
    max-width: 1200px;
    margin: 0 auto;
    padding: clamp(1rem, 2.5vw, 2rem);
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .page-header h1 {
    margin: 0;
    font-size: clamp(1.25rem, 2vw, 1.6rem);
  }

  .subtitle {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
  }

  .header-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    flex-wrap: wrap;
  }

  .card {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 1rem;
  }

  .controls {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
    align-items: end;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .field label {
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 600;
  }

  .field select {
    border: 1px solid var(--border-color);
    background: var(--bg-input);
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.65rem 0.75rem;
    min-height: 40px;
  }

  .current-status {
    border: 1px dashed var(--border-color);
    border-radius: 10px;
    padding: 0.7rem;
    min-height: 40px;
  }

  .status-label {
    color: var(--text-secondary);
    font-size: 0.78rem;
    margin-bottom: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .status-value {
    font-weight: 700;
  }

  .status-sub,
  .status-empty {
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .package-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 0.9rem;
  }

  .package-card {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .package-card.active {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-primary) 40%, transparent);
  }

  .package-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .package-top h3 {
    margin: 0;
    font-size: 1rem;
    line-height: 1.3;
  }

  .description {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
    min-height: 2.6em;
  }

  .price {
    font-size: 1.4rem;
    font-weight: 800;
    letter-spacing: -0.02em;
  }

  .price-sub {
    margin-top: -0.3rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .features {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  .features li {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .badge {
    background: color-mix(in srgb, var(--accent-primary) 18%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent-primary) 50%, transparent);
    color: var(--accent-primary);
    border-radius: 999px;
    padding: 0.12rem 0.5rem;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    border-radius: 10px;
    border: 1px solid transparent;
    padding: 0.6rem 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-secondary);
    border-color: var(--border-color);
    color: var(--text-primary);
  }

  .btn-primary {
    background: var(--accent-primary);
    color: var(--text-on-primary, #fff);
  }

  .checkout-modal {
    display: grid;
    gap: 0.8rem;
  }

  .checkout-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.6rem;
  }

  .checkout-summary > div {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.6rem 0.65rem;
    display: grid;
    gap: 0.2rem;
    background: var(--bg-secondary);
  }

  .checkout-summary small {
    color: var(--text-secondary);
    font-size: 0.75rem;
  }

  .checkout-summary strong {
    font-size: 0.92rem;
  }

  .checkout-note {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.45;
  }

  .cycle-pills {
    display: inline-flex;
    gap: 0.5rem;
    align-items: center;
  }

  .cycle-pill {
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 0.4rem 0.75rem;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .cycle-pill.active {
    border-color: color-mix(in srgb, var(--accent-primary) 55%, var(--border-color));
    background: color-mix(in srgb, var(--accent-primary) 14%, transparent);
    color: var(--accent-primary);
    font-weight: 700;
  }

  .checkout-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
  }

  .empty-block {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .empty-block h3 {
    margin: 0;
    font-size: 1rem;
  }

  .empty-block p {
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
  }

  .empty-actions {
    margin-top: 0.75rem;
  }

  .location-form {
    display: grid;
    gap: 0.85rem;
  }

  .form-field {
    display: grid;
    gap: 0.35rem;
  }

  .form-field > span {
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-input);
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.65rem 0.75rem;
    min-height: 40px;
  }

  .textarea {
    min-height: 84px;
    resize: vertical;
  }

  .location-grid-2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.65rem;
  }

  .skeleton {
    min-height: 220px;
    opacity: 0.4;
  }

  @media (max-width: 900px) {
    .controls {
      grid-template-columns: 1fr;
    }
    .checkout-summary {
      grid-template-columns: 1fr;
    }
    .location-grid-2 {
      grid-template-columns: 1fr;
    }
  }
</style>
