<script lang="ts">
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import { toast } from 'svelte-sonner';
  import { api, type Customer, type CustomerLocation, type CustomerListItem, type IspPackage } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { getVisibleInternetOrderPackages } from '$lib/utils/internetOrderPackages';
  import { getAdminCustomerNavigation } from '$lib/utils/adminCustomerNavigation';
  import {
    buildBackofficeInstallationOrderPayload,
    inferInitialCustomerMode,
    type OrderWizardDraft,
  } from './orderWizardState';

  type Step = 1 | 2 | 3;

  let loading = $state(true);
  let submitting = $state(false);
  let step = $state<Step>(1);
  let customerSearch = $state('');
  let customerResults = $state<CustomerListItem[]>([]);
  let customerSearchLoading = $state(false);
  let selectedCustomer = $state<Customer | null>(null);
  let locations = $state<CustomerLocation[]>([]);
  let packages = $state<IspPackage[]>([]);

  const canCreateOrders = $derived($can('create', 'orders'));
  const canReadWorkOrders = $derived($can('read', 'work_orders') || $can('manage', 'work_orders'));
  const customerNav = $derived.by(() =>
    getAdminCustomerNavigation({
      hostname: $pageStore.url.hostname,
      tenantSlug: $pageStore.data?.tenant?.slug,
      routeTenantSlug: $pageStore.params.tenant,
    }),
  );
  const customersPath = $derived(customerNav.customersPath);

  let draft = $state<OrderWizardDraft>({
    customerMode: 'new',
    existingCustomerId: '',
    customer: {
      name: '',
      email: '',
      phone: '',
      notes: '',
      is_active: true,
    },
    locationMode: 'new',
    existingLocationId: '',
    location: {
      label: '',
      address_line1: '',
      address_line2: '',
      city: '',
      state: '',
      postal_code: '',
      country: 'ID',
      latitude: '',
      longitude: '',
      notes: '',
    },
    packageId: '',
    billingCycle: 'monthly',
    notes: '',
    requestedInstallationDate: '',
  });

  const selectedPackage = $derived.by(() => packages.find((pkg) => pkg.id === draft.packageId) || null);

  onMount(() => {
    void init();
  });

  async function init() {
    if (!canCreateOrders) {
      goto('/unauthorized');
      return;
    }

    loading = true;
    try {
      const prefilledCustomerId = get(pageStore).url.searchParams.get('customer_id');
      draft.customerMode = inferInitialCustomerMode(prefilledCustomerId);
      draft.existingCustomerId = prefilledCustomerId || '';

      const packageResponse = await api.ispPackages.packages.list({ page: 1, per_page: 200, q: '' });
      packages = getVisibleInternetOrderPackages((packageResponse?.data || []).filter((pkg) => pkg.is_active));
      if (!draft.packageId && packages.length > 0) {
        draft.packageId = packages[0].id;
      }

      if (prefilledCustomerId) {
        selectedCustomer = await api.customers.get(prefilledCustomerId);
        await loadLocations(prefilledCustomerId);
      }
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load order wizard');
    } finally {
      loading = false;
    }
  }

  async function searchCustomers() {
    if (draft.customerMode !== 'existing') return;
    const query = customerSearch.trim();
    if (query.length < 2) {
      customerResults = [];
      return;
    }

    customerSearchLoading = true;
    try {
      const result = await api.customers.list({ q: query, page: 1, perPage: 10 });
      customerResults = result.data || [];
    } catch (e: any) {
      toast.error(e?.message || 'Failed to search customers');
    } finally {
      customerSearchLoading = false;
    }
  }

  async function selectCustomer(customerId: string) {
    draft.existingCustomerId = customerId;
    selectedCustomer = await api.customers.get(customerId);
    draft.locationMode = 'existing';
    draft.existingLocationId = '';
    await loadLocations(customerId);
  }

  async function loadLocations(customerId: string) {
    locations = await api.customers.locations.list(customerId);
    if (locations.length > 0 && !draft.existingLocationId) {
      draft.existingLocationId = locations[0].id;
    }
  }

  function nextStep() {
    try {
      if (step === 1) {
        if (draft.customerMode === 'existing' && !draft.existingCustomerId.trim()) {
          throw new Error('Select an existing customer first');
        }
        if (draft.customerMode === 'new') {
          if (!draft.customer.name.trim()) throw new Error('Customer name is required');
          if (!draft.customer.email.trim() && !draft.customer.phone.trim()) {
            throw new Error('Customer email or phone is required');
          }
        }
        step = 2;
        return;
      }

      if (step === 2) {
        if (draft.locationMode === 'existing' && !draft.existingLocationId.trim()) {
          throw new Error('Select an existing address first');
        }
        if (draft.locationMode === 'new') {
          if (!draft.location.label.trim()) throw new Error('Location label is required');
          if (!draft.location.address_line1.trim()) throw new Error('Address line 1 is required');
        }
        if (!draft.packageId.trim()) throw new Error('Package is required');
        step = 3;
      }
    } catch (e: any) {
      toast.error(e?.message || 'Please complete the form first');
    }
  }

  function prevStep() {
    step = step === 3 ? 2 : 1;
  }

  async function submitOrder() {
    submitting = true;
    try {
      const payload = buildBackofficeInstallationOrderPayload(draft);
      const result = await api.customers.orders.createInstallation(payload);
      toast.success('Installation order created');

      if (canReadWorkOrders && result.work_order?.id) {
        goto(`/admin/network/installations?work_order_id=${encodeURIComponent(result.work_order.id)}`);
        return;
      }

      goto(`${customersPath}/${result.customer.id}`);
    } catch (e: any) {
      toast.error(e?.message || 'Failed to create installation order');
    } finally {
      submitting = false;
    }
  }

  function packagePriceLabel(pkg: IspPackage) {
    const amount = draft.billingCycle === 'yearly' && Number(pkg.price_yearly || 0) > 0
      ? Number(pkg.price_yearly || 0)
      : Number(pkg.price_monthly || 0);
    return new Intl.NumberFormat('id-ID', { style: 'currency', currency: 'IDR' }).format(amount);
  }

  function formatSelectedCustomer() {
    if (draft.customerMode === 'new') return draft.customer.name || '-';
    return selectedCustomer?.name || draft.existingCustomerId || '-';
  }

  function formatSelectedLocation() {
    if (draft.locationMode === 'new') return draft.location.label || draft.location.address_line1 || '-';
    return locations.find((location) => location.id === draft.existingLocationId)?.label || draft.existingLocationId || '-';
  }
</script>

<svelte:head>
  <title>Create Installation Order</title>
</svelte:head>

{#if loading}
  <div class="page-shell"><div class="card">Loading order wizard...</div></div>
{:else}
  <div class="page-shell">
    <div class="page-header">
      <div>
        <p class="eyebrow">Backoffice Order</p>
        <h1>Create Installation Order</h1>
        <p class="subtle">Create customer, address, service, and installation work order from one flow.</p>
      </div>
    </div>

    <div class="stepper">
      <div class:active={step === 1}>1. Customer</div>
      <div class:active={step === 2}>2. Address & Service</div>
      <div class:active={step === 3}>3. Review</div>
    </div>

    {#if step === 1}
      <section class="card section">
        <div class="mode-row">
          <button class:active-mode={draft.customerMode === 'new'} class="mode-btn" onclick={() => (draft.customerMode = 'new')}>New Customer</button>
          <button class:active-mode={draft.customerMode === 'existing'} class="mode-btn" onclick={() => (draft.customerMode = 'existing')}>Existing Customer</button>
        </div>

        {#if draft.customerMode === 'existing'}
          <div class="grid two">
            <label>
              <span>Search customer</span>
              <div class="inline-search">
                <input class="input" bind:value={customerSearch} placeholder="Name, email, phone" />
                <button class="btn btn-secondary" onclick={searchCustomers} disabled={customerSearchLoading}>Search</button>
              </div>
            </label>
          </div>

          <div class="search-results">
            {#if customerResults.length === 0}
              <div class="subtle">Search with at least 2 characters to load existing customers.</div>
            {:else}
              {#each customerResults as customer}
                <button class:selected={draft.existingCustomerId === customer.id} class="result-card" onclick={() => void selectCustomer(customer.id)}>
                  <strong>{customer.name}</strong>
                  <span>{customer.phone || customer.email || 'No contact'}</span>
                </button>
              {/each}
            {/if}
          </div>
        {:else}
          <div class="grid two">
            <label><span>Name</span><input class="input" bind:value={draft.customer.name} /></label>
            <label><span>Phone</span><input class="input" bind:value={draft.customer.phone} /></label>
            <label><span>Email</span><input class="input" bind:value={draft.customer.email} /></label>
            <label class="checkbox-row"><input type="checkbox" bind:checked={draft.customer.is_active} /> <span>Customer active</span></label>
          </div>
          <label><span>Notes</span><textarea class="input" rows="4" bind:value={draft.customer.notes}></textarea></label>
        {/if}
      </section>
    {/if}

    {#if step === 2}
      <section class="card section">
        <div class="mode-row">
          <button class:active-mode={draft.locationMode === 'existing'} class="mode-btn" disabled={draft.customerMode === 'new' && !draft.customer.name.trim()} onclick={() => (draft.locationMode = 'existing')}>Use Existing Address</button>
          <button class:active-mode={draft.locationMode === 'new'} class="mode-btn" onclick={() => (draft.locationMode = 'new')}>Add New Address</button>
        </div>

        {#if draft.locationMode === 'existing'}
          <label>
            <span>Customer address</span>
            <select class="input" bind:value={draft.existingLocationId}>
              <option value="">Select address</option>
              {#each locations as location}
                <option value={location.id}>{location.label} - {location.address_line1 || 'No address line'}</option>
              {/each}
            </select>
          </label>
        {:else}
          <div class="grid two">
            <label><span>Label</span><input class="input" bind:value={draft.location.label} /></label>
            <label><span>Address line 1</span><input class="input" bind:value={draft.location.address_line1} /></label>
            <label><span>Address line 2</span><input class="input" bind:value={draft.location.address_line2} /></label>
            <label><span>City</span><input class="input" bind:value={draft.location.city} /></label>
            <label><span>State</span><input class="input" bind:value={draft.location.state} /></label>
            <label><span>Postal code</span><input class="input" bind:value={draft.location.postal_code} /></label>
            <label><span>Country</span><input class="input" bind:value={draft.location.country} /></label>
            <label><span>Latitude</span><input class="input" bind:value={draft.location.latitude} /></label>
            <label><span>Longitude</span><input class="input" bind:value={draft.location.longitude} /></label>
          </div>
          <label><span>Location notes</span><textarea class="input" rows="3" bind:value={draft.location.notes}></textarea></label>
        {/if}

        <div class="grid two">
          <label>
            <span>Package</span>
            <select class="input" bind:value={draft.packageId}>
              <option value="">Select package</option>
              {#each packages as pkg}
                <option value={pkg.id}>{pkg.name} - {packagePriceLabel(pkg)}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Billing cycle</span>
            <select class="input" bind:value={draft.billingCycle}>
              <option value="monthly">Monthly</option>
              <option value="yearly">Yearly</option>
            </select>
          </label>
        </div>

        <div class="grid two">
          <label><span>Requested installation date</span><input class="input" type="datetime-local" bind:value={draft.requestedInstallationDate} /></label>
          <label><span>Order notes</span><input class="input" bind:value={draft.notes} /></label>
        </div>
      </section>
    {/if}

    {#if step === 3}
      <section class="card section summary">
        <div><span>Customer</span><strong>{formatSelectedCustomer()}</strong></div>
        <div><span>Address</span><strong>{formatSelectedLocation()}</strong></div>
        <div><span>Package</span><strong>{selectedPackage?.name || '-'}</strong></div>
        <div><span>Billing</span><strong>{draft.billingCycle}</strong></div>
        <div><span>Requested install</span><strong>{draft.requestedInstallationDate || '-'}</strong></div>
        <div><span>Notes</span><strong>{draft.notes || '-'}</strong></div>
      </section>
    {/if}

    <div class="actions">
      <button class="btn btn-secondary" onclick={() => goto(customersPath)}>Cancel</button>
      {#if step > 1}
        <button class="btn btn-secondary" onclick={prevStep}>Back</button>
      {/if}
      {#if step < 3}
        <button class="btn btn-primary" onclick={nextStep}>Continue</button>
      {:else}
        <button class="btn btn-primary" onclick={submitOrder} disabled={submitting}>{submitting ? 'Submitting...' : 'Create Order'}</button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .page-shell { display: grid; gap: 1rem; padding: 1.25rem; }
  .page-header { display: flex; justify-content: space-between; align-items: flex-start; }
  .eyebrow { margin: 0 0 0.25rem; text-transform: uppercase; letter-spacing: 0.12em; color: #8a5a2b; font-size: 0.72rem; }
  h1 { margin: 0; font-size: 2rem; }
  .subtle { color: #6b7280; }
  .card { background: white; border: 1px solid #e5e7eb; border-radius: 18px; padding: 1rem; box-shadow: 0 14px 35px rgba(15, 23, 42, 0.06); }
  .section { display: grid; gap: 1rem; }
  .stepper { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.75rem; }
  .stepper div { border-radius: 999px; border: 1px solid #d1d5db; padding: 0.7rem 0.9rem; text-align: center; color: #6b7280; }
  .stepper div.active { background: #0f766e; border-color: #0f766e; color: white; }
  .grid.two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 1rem; }
  label { display: grid; gap: 0.4rem; color: #111827; font-weight: 600; }
  .input { width: 100%; border: 1px solid #d1d5db; border-radius: 12px; padding: 0.75rem 0.9rem; font: inherit; background: white; }
  textarea.input { resize: vertical; }
  .mode-row { display: flex; gap: 0.75rem; flex-wrap: wrap; }
  .mode-btn { border: 1px solid #d1d5db; border-radius: 999px; background: #fff; padding: 0.6rem 1rem; }
  .mode-btn.active-mode { border-color: #0f766e; background: #ecfdf5; color: #0f766e; }
  .checkbox-row { display: flex; align-items: center; gap: 0.6rem; padding-top: 1.8rem; }
  .inline-search { display: flex; gap: 0.75rem; }
  .search-results { display: grid; gap: 0.75rem; }
  .result-card { display: grid; gap: 0.2rem; text-align: left; border: 1px solid #e5e7eb; border-radius: 14px; padding: 0.85rem 1rem; background: #fff; }
  .result-card.selected { border-color: #0f766e; background: #f0fdfa; }
  .summary { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .summary div { display: grid; gap: 0.25rem; padding: 0.9rem; border: 1px solid #e5e7eb; border-radius: 14px; }
  .summary span { color: #6b7280; font-size: 0.8rem; }
  .actions { display: flex; gap: 0.75rem; justify-content: flex-end; flex-wrap: wrap; }
  .btn { border: 0; border-radius: 999px; padding: 0.75rem 1.2rem; font: inherit; cursor: pointer; }
  .btn-primary { background: #111827; color: white; }
  .btn-secondary { background: #f3f4f6; color: #111827; }
  @media (max-width: 768px) {
    .grid.two, .summary, .stepper { grid-template-columns: 1fr; }
    .inline-search, .actions { flex-direction: column; }
  }
</style>
