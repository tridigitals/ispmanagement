<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
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

  async function checkout(pkg: IspPackage) {
    if (!selectedLocationId) {
      toast.error(get(t)('dashboard.packages.toasts.select_location') || 'Select a location first');
      return;
    }

    if (selectedCycle === 'yearly' && !hasYearlyPrice(pkg)) {
      toast.error(
        get(t)('dashboard.packages.toasts.yearly_unavailable') ||
          'Yearly billing is not available for this package',
      );
      return;
    }

    if (
      currentSubscription?.package_id === pkg.id &&
      currentSubscription?.billing_cycle === selectedCycle
    ) {
      toast.info(
        get(t)('dashboard.packages.toasts.already_active') || 'This package is already active',
      );
      return;
    }

    purchasingPackageId = pkg.id;
    try {
      const res = await api.customers.portal.checkout({
        location_id: selectedLocationId,
        package_id: pkg.id,
        billing_cycle: selectedCycle,
      });
      toast.success(
        get(t)('dashboard.packages.toasts.checkout_success') ||
          'Invoice created. Continue to payment.',
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
    <button class="btn btn-secondary" onclick={loadData} disabled={loading}>
      <Icon name="refresh-cw" size={16} />
      {$t('common.refresh') || 'Refresh'}
    </button>
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

    <div class="field">
      <label for="billing-cycle"
        >{$t('dashboard.packages.fields.billing_cycle') || 'Billing cycle'}</label
      >
      <select id="billing-cycle" bind:value={selectedCycle} disabled={loading}>
        <option value="monthly">{$t('dashboard.packages.cycles.monthly') || 'Monthly'}</option>
        <option value="yearly">{$t('dashboard.packages.cycles.yearly') || 'Yearly'}</option>
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

          <div class="price">{formatCurrency(getPrice(pkg))}</div>
          <div class="price-sub">
            {#if selectedCycle === 'yearly'}
              {$t('dashboard.packages.cycles.yearly') || 'Yearly'}
            {:else}
              {$t('dashboard.packages.cycles.monthly') || 'Monthly'}
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
              purchasingPackageId === pkg.id ||
              (selectedCycle === 'yearly' && !hasYearlyPrice(pkg))}
            onclick={() => checkout(pkg)}
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

  .card {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 1rem;
  }

  .controls {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
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

  .skeleton {
    min-height: 220px;
    opacity: 0.4;
  }

  @media (max-width: 900px) {
    .controls {
      grid-template-columns: 1fr;
    }
  }
</style>
