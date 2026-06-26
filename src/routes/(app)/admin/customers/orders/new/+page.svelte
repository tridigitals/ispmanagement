<script lang="ts">
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { onDestroy, onMount, tick } from 'svelte';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { t } from 'svelte-i18n';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import { api, type Customer, type CustomerLocation, type CustomerListItem, type IspPackage } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import { getVisibleInternetOrderPackages } from '$lib/utils/internetOrderPackages';
  import { getAdminCustomerNavigation } from '$lib/utils/adminCustomerNavigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { buildCountryOptions } from './countryOptions';
  import { getCustomerSearchViewState } from './customerSearchState';
  import {
    buildPhonePrefixOptions,
    composePhoneNumber,
    inferPhoneFieldState,
  } from './phoneField';
  import {
    buildBackofficeInstallationOrderPayload,
    inferInitialCustomerMode,
    type OrderWizardDraft,
  } from './orderWizardState';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type Step = 1 | 2 | 3;

  let loading = $state(true);
  let submitting = $state(false);
  let step = $state<Step>(1);
  let customerSearch = $state('');
  let customerResults = $state<CustomerListItem[]>([]);
  let customerSearchLoading = $state(false);
  let customerSearchHasSearched = $state(false);
  let selectedCustomer = $state<Customer | null>(null);
  let locations = $state<CustomerLocation[]>([]);
  let packages = $state<IspPackage[]>([]);
  const countryOptions = buildCountryOptions();
  const phonePrefixOptions = buildPhonePrefixOptions();
  let phonePrefix = $state('+62');
  let phoneLocalNumber = $state('');
  let showMapPicker = $state(false);
  let pickerMapHost = $state<HTMLDivElement | null>(null);
  let pickerMap: any = null;
  let pickerMarker: any = null;
  let pickerLat = $state<number | null>(null);
  let pickerLng = $state<number | null>(null);
  let maplibrePromise: Promise<any> | null = null;
  let pickerViewMode = $state<'standard' | 'satellite'>('standard');
  let pickerMapLoading = $state(false);
  let pickerMapUnavailable = $state(false);
  let pickerMapErrorMessage = $state('');
  const pickerMapTilerKey = (import.meta.env.VITE_MAPTILER_KEY as string | undefined)?.trim();
  const pickerStandardMaxZoom = 19;
  const pickerSatelliteMaxZoom = pickerMapTilerKey ? 21 : 18;

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
  const customerSearchView = $derived.by(() =>
    getCustomerSearchViewState({
      query: customerSearch,
      loading: customerSearchLoading,
      hasSearched: customerSearchHasSearched,
      resultCount: customerResults.length,
    }),
  );

  onMount(() => {
    const initialPhoneState = inferPhoneFieldState(draft.customer.phone);
    phonePrefix = initialPhoneState.prefix;
    phoneLocalNumber = initialPhoneState.localNumber;
    void init();
  });

  onDestroy(() => {
    pickerMarker?.remove();
    pickerMap?.remove();
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
      toast.error(e?.message || $t('admin.network.installations.toasts.load_wizard_failed'));
    } finally {
      loading = false;
    }
  }

  async function searchCustomers() {
    if (draft.customerMode !== 'existing') return;
    const query = customerSearch.trim();
    if (query.length < 2) {
      customerSearchHasSearched = false;
      customerResults = [];
      return;
    }

    customerSearchHasSearched = true;
    customerSearchLoading = true;
    try {
      const result = await api.customers.list({ q: query, page: 1, perPage: 10 });
      customerResults = result.data || [];
    } catch (e: any) {
      toast.error(e?.message || $t('admin.network.installations.toasts.search_customers_failed'));
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
          throw new Error($t('admin.network.installations.validation.select_existing_customer'));
        }
        if (draft.customerMode === 'new') {
          if (!draft.customer.name.trim()) throw new Error($t('admin.network.installations.validation.customer_name_required'));
          if (!draft.customer.email.trim() && !draft.customer.phone.trim()) {
            throw new Error($t('admin.network.installations.validation.customer_email_or_phone_required'));
          }
        }
        step = 2;
        return;
      }

      if (step === 2) {
        if (draft.locationMode === 'existing' && !draft.existingLocationId.trim()) {
          throw new Error($t('admin.network.installations.validation.select_existing_address'));
        }
        if (draft.locationMode === 'new') {
          if (!draft.location.label.trim()) throw new Error($t('admin.network.installations.validation.location_label_required'));
          if (!draft.location.address_line1.trim()) throw new Error($t('admin.network.installations.validation.address_line1_required'));
        }
        if (!draft.packageId.trim()) throw new Error($t('admin.network.installations.validation.package_required'));
        step = 3;
      }
    } catch (e: any) {
      toast.error(e?.message || $t('admin.network.installations.validation.complete_form'));
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
      toast.success($t('admin.network.installations.toasts.order_created'));

      if (canReadWorkOrders && result.work_order?.id) {
        goto(`/admin/network/installations?work_order_id=${encodeURIComponent(result.work_order.id)}`);
        return;
      }

      goto(`${customersPath}/${result.customer.id}`);
    } catch (e: any) {
      toast.error(e?.message || $t('admin.network.installations.toasts.order_create_failed'));
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

  function parseCoordOrNull(value: string) {
    const raw = value.trim();
    if (!raw) return null;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : NaN;
  }

  async function getMaplibre() {
    if (!maplibrePromise) {
      maplibrePromise = import('maplibre-gl');
    }
    return maplibrePromise;
  }

  function setPickerPoint(lat: number, lng: number) {
    pickerLat = lat;
    pickerLng = lng;
    if (pickerMarker) {
      pickerMarker.setLngLat([lng, lat]);
      return;
    }
    if (!pickerMap) return;
    pickerMarker = new (pickerMap as any).libregl.Marker({ draggable: true })
      .setLngLat([lng, lat])
      .addTo(pickerMap);
    pickerMarker.on('dragend', () => {
      const pos = pickerMarker.getLngLat();
      pickerLat = Number(pos.lat.toFixed(7));
      pickerLng = Number(pos.lng.toFixed(7));
    });
  }

  function syncPickerViewMode() {
    if (!pickerMap) return;
    const showSatellite = pickerViewMode === 'satellite';
    const setVisibility = (layerId: string, visible: boolean) => {
      if (!pickerMap.getLayer(layerId)) return;
      pickerMap.setLayoutProperty(layerId, 'visibility', visible ? 'visible' : 'none');
    };
    setVisibility('order-picker-standard', !showSatellite);
    setVisibility('order-picker-satellite', showSatellite);
    const targetMaxZoom = showSatellite ? pickerSatelliteMaxZoom : pickerStandardMaxZoom;
    pickerMap.setMaxZoom(targetMaxZoom);
    if (pickerMap.getZoom() > targetMaxZoom) {
      pickerMap.setZoom(targetMaxZoom);
    }
  }

  async function openMapPicker() {
    const initialLat = parseCoordOrNull(draft.location.latitude);
    const initialLng = parseCoordOrNull(draft.location.longitude);
    const nextLat: number =
      typeof initialLat === 'number' && Number.isFinite(initialLat) ? initialLat : -6.2;
    const nextLng: number =
      typeof initialLng === 'number' && Number.isFinite(initialLng) ? initialLng : 106.816666;
    pickerLat = nextLat;
    pickerLng = nextLng;
    pickerMapUnavailable = false;
    pickerMapErrorMessage = '';
    showMapPicker = true;
    await tick();
    if (!pickerMapHost) return;

    pickerMapLoading = true;
    try {
      const libregl = await getMaplibre();
      if (!pickerMap) {
        pickerMap = new libregl.Map({
          container: pickerMapHost,
          style: {
            version: 8,
            sources: {
              standard: {
                type: 'raster',
                tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
                tileSize: 256,
                attribution: 'OpenStreetMap contributors',
                maxzoom: pickerStandardMaxZoom,
              },
              satellite: {
                type: 'raster',
                tiles: pickerMapTilerKey
                  ? [
                      `https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=${pickerMapTilerKey}`,
                    ]
                  : [
                      'https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
                    ],
                tileSize: 256,
                attribution: pickerMapTilerKey ? 'MapTiler' : 'Esri',
                maxzoom: pickerSatelliteMaxZoom,
              },
            },
            layers: [
              { id: 'order-picker-standard', type: 'raster', source: 'standard' },
              {
                id: 'order-picker-satellite',
                type: 'raster',
                source: 'satellite',
                layout: { visibility: 'none' },
              },
            ],
          },
          center: [nextLng, nextLat],
          zoom: 13,
          maxZoom: pickerStandardMaxZoom,
        });
        (pickerMap as any).libregl = libregl;
        pickerMap.addControl(
          new libregl.NavigationControl({ showCompass: true, showZoom: true }),
          'top-right',
        );
        pickerMap.addControl(
          new libregl.GeolocateControl({ trackUserLocation: false, showAccuracyCircle: true }),
          'top-right',
        );
        pickerMap.on('click', (event: any) => {
          const { lat, lng } = event.lngLat;
          setPickerPoint(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
        });
      } else {
        pickerMap.resize();
        pickerMap.setCenter([nextLng, nextLat]);
        pickerMap.setZoom(
          Math.min(
            13,
            pickerViewMode === 'satellite' ? pickerSatelliteMaxZoom : pickerStandardMaxZoom,
          ),
        );
      }
      syncPickerViewMode();
      setPickerPoint(nextLat, nextLng);
    } catch (e: any) {
      pickerMapUnavailable = true;
      pickerMapErrorMessage = e?.message || $t('admin.network.installations.toasts.map_init_failed');
    } finally {
      pickerMapLoading = false;
      pickerMap?.resize();
    }
  }

  function closeMapPicker() {
    showMapPicker = false;
  }

  function onPickerSearchSelect(event: CustomEvent<{ lat: number; lng: number }>) {
    const { lat, lng } = event.detail;
    setPickerPoint(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
    pickerMap?.flyTo({
      center: [lng, lat],
      zoom: Math.max(pickerMap.getZoom(), 15),
      duration: 480,
    });
  }

  function applyPickedCoordinates() {
    if (!Number.isFinite(pickerLat) || !Number.isFinite(pickerLng)) {
      toast.error($t('admin.network.installations.toasts.pick_location_first'));
      return;
    }
    draft.location.latitude = String(pickerLat);
    draft.location.longitude = String(pickerLng);
    closeMapPicker();
  }

  $effect(() => {
    pickerViewMode;
    if (!pickerMap) return;
    syncPickerViewMode();
  });

  $effect(() => {
    draft.customer.phone = composePhoneNumber(phonePrefix, phoneLocalNumber);
  });

  function handleCustomerSearchKeydown(event: KeyboardEvent) {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    void searchCustomers();
  }
</script>

<svelte:head>
  <title>{$t('admin.network.installations.create_order')}</title>
</svelte:head>

{#if loading}
  <div class="page-content"><div class="hero-card">{$t('admin.network.installations.loading_wizard')}</div></div>
{:else}
  <div class="page-content fade-in">
    <section class="hero-card">
      <div class="hero-copy">
        <h1>{$t('admin.network.installations.create_order')}</h1>
        <p class="subtitle">{$t('admin.network.installations.order_subtitle')}</p>
      </div>
    </section>

    <div class="stepper">
      <div class:active={step === 1}>
        <span class="step-index">1</span>
        <span>{$t('common.customer')}</span>
      </div>
      <div class:active={step === 2}>
        <span class="step-index">2</span>
        <span>{$t('admin.network.installations.address_service')}</span>
      </div>
      <div class:active={step === 3}>
        <span class="step-index">3</span>
        <span>{$t('common.review')}</span>
      </div>
    </div>

    {#if step === 1}
      <section class="form-card section">
        <div class="section-header">
          <div>
            <h2>{$t('admin.network.installations.customer_context')}</h2>
            <p>{$t('admin.network.installations.customer_context_hint')}</p>
          </div>
        </div>

        <div class="segmented">
          <button class:active-mode={draft.customerMode === 'new'} class="mode-btn" onclick={() => (draft.customerMode = 'new')}>
            <Icon name="plus" size={15} />
            {$t('admin.network.installations.new_customer')}
          </button>
          <button class:active-mode={draft.customerMode === 'existing'} class="mode-btn" onclick={() => (draft.customerMode = 'existing')}>
            <Icon name="users" size={15} />
            {$t('admin.network.installations.existing_customer')}
          </button>
        </div>

        {#if draft.customerMode === 'existing'}
          <div class="form-grid">
            <label>
              <span>{$t('admin.network.installations.search_customer')}</span>
              <div class="inline-search">
                <Icon name="search" size={16} />
                <input
                  class="input"
                  bind:value={customerSearch}
                  placeholder={$t('admin.network.installations.search_customer_placeholder')}
                  onkeydown={handleCustomerSearchKeydown}
                />
                <button
                  class="btn btn-secondary"
                  type="button"
                  onclick={searchCustomers}
                  disabled={customerSearchLoading || customerSearch.trim().length < 2}
                >
                  {customerSearchLoading ? $t('admin.network.installations.searching') : $t('common.search')}
                </button>
              </div>
            </label>
          </div>

          <div class="search-results">
            {#if customerSearchView.kind !== 'results'}
              <div class="empty-state">
                <Icon name="search" size={16} />
                <span>{customerSearchView.message}</span>
              </div>
            {:else}
              {#each customerResults as customer}
                <button class:selected={draft.existingCustomerId === customer.id} class="result-card" onclick={() => void selectCustomer(customer.id)}>
                  <div class="result-main">
                    <strong>{customer.name}</strong>
                    <span>{customer.phone || customer.email || $t('admin.network.installations.no_contact')}</span>
                  </div>
                  <span class="result-status">{customer.pending_installations > 0 ? $t('admin.network.installations.pending_install') : customer.service_status || $t('common.customer')}</span>
                </button>
              {/each}
            {/if}
          </div>
        {:else}
          <div class="form-grid">
            <label><span>{$t('common.name')}</span><input class="input" bind:value={draft.customer.name} /></label>
            <label>
              <span>{$t('common.phone')}</span>
              <div class="phone-field">
                <div class="phone-prefix">
                  <Select2
                    bind:value={phonePrefix}
                    options={phonePrefixOptions}
                    placeholder={$t('admin.network.installations.phone_code')}
                    width="100%"
                    searchPlaceholder={$t('admin.network.installations.search_country_code')}
                    noResultsText={$t('admin.network.installations.no_code_found')}
                    maxItems={300}
                  />
                </div>
                <input class="input" bind:value={phoneLocalNumber} placeholder="8123456789" inputmode="tel" />
              </div>
            </label>
            <label><span>{$t('common.email')}</span><input class="input" bind:value={draft.customer.email} /></label>
            <label class="checkbox-row"><input type="checkbox" bind:checked={draft.customer.is_active} /> <span>{$t('admin.network.installations.customer_active')}</span></label>
          </div>
          <label><span>{$t('common.notes')}</span><textarea class="input" rows="4" bind:value={draft.customer.notes}></textarea></label>
        {/if}
      </section>
    {/if}

    {#if step === 2}
      <section class="form-card section">
        <div class="section-header">
          <div>
            <h2>{$t('admin.network.installations.address_service')}</h2>
            <p>{$t('admin.network.installations.address_service_hint')}</p>
          </div>
        </div>

        <div class="segmented">
          <button class:active-mode={draft.locationMode === 'existing'} class="mode-btn" disabled={draft.customerMode === 'new' && !draft.customer.name.trim()} onclick={() => (draft.locationMode = 'existing')}>
            <Icon name="map-pin" size={15} />
            {$t('admin.network.installations.use_existing_address')}
          </button>
          <button class:active-mode={draft.locationMode === 'new'} class="mode-btn" onclick={() => (draft.locationMode = 'new')}>
            <Icon name="plus" size={15} />
            {$t('admin.network.installations.add_new_address')}
          </button>
        </div>

        {#if draft.locationMode === 'existing'}
          <label>
            <span>{$t('admin.network.installations.customer_address')}</span>
            <select class="input" bind:value={draft.existingLocationId}>
              <option value="">{$t('admin.network.installations.select_address')}</option>
              {#each locations as location}
                <option value={location.id}>{location.label} - {location.address_line1 || $t('admin.network.installations.no_address_line')}</option>
              {/each}
            </select>
          </label>
        {:else}
          <div class="form-grid">
            <label><span>{$t('admin.network.installations.location_label')}</span><input class="input" bind:value={draft.location.label} /></label>
            <label><span>{$t('admin.network.installations.address_line1')}</span><input class="input" bind:value={draft.location.address_line1} /></label>
            <label><span>{$t('admin.network.installations.address_line2')}</span><input class="input" bind:value={draft.location.address_line2} /></label>
            <label><span>{$t('admin.network.installations.city')}</span><input class="input" bind:value={draft.location.city} /></label>
            <label><span>{$t('admin.network.installations.state')}</span><input class="input" bind:value={draft.location.state} /></label>
            <label><span>{$t('admin.network.installations.postal_code')}</span><input class="input" bind:value={draft.location.postal_code} /></label>
            <label>
              <span>{$t('admin.network.installations.country')}</span>
              <Select2
                bind:value={draft.location.country}
                options={countryOptions}
                placeholder={$t('admin.network.installations.select_country')}
                width="100%"
                searchPlaceholder={$t('admin.network.installations.search_country')}
                noResultsText={$t('admin.network.installations.no_country_found')}
                maxItems={500}
              />
            </label>
          </div>

          <div class="map-picked-box">
            <div>
              <div class="map-picked-title">{$t('admin.network.installations.installation_point')}</div>
              <div class="map-picked-sub">{$t('admin.network.installations.installation_point_hint')}</div>
            </div>
            <button class="btn btn-secondary" type="button" onclick={openMapPicker}>
              <Icon name="map" size={16} />
              {draft.location.latitude && draft.location.longitude ? $t('admin.network.installations.update_map_point') : $t('admin.network.installations.pick_map_point')}
            </button>
          </div>

          <div class="form-grid">
            <label><span>{$t('admin.network.installations.latitude')}</span><input class="input mono-input" bind:value={draft.location.latitude} readonly /></label>
            <label><span>{$t('admin.network.installations.longitude')}</span><input class="input mono-input" bind:value={draft.location.longitude} readonly /></label>
          </div>
          <label><span>{$t('admin.network.installations.location_notes')}</span><textarea class="input" rows="3" bind:value={draft.location.notes}></textarea></label>
        {/if}

        <div class="context-grid">
          <div class="context-card">
            <span class="context-label">{$t('common.customer')}</span>
            <strong>{formatSelectedCustomer()}</strong>
          </div>
          <div class="context-card">
            <span class="context-label">{$t('admin.network.installations.location_mode')}</span>
            <strong>{draft.locationMode === 'existing' ? $t('admin.network.installations.use_saved_address') : $t('admin.network.installations.create_new_address')}</strong>
          </div>
        </div>

        <div class="form-grid">
          <label>
            <span>{$t('common.package')}</span>
            <select class="input" bind:value={draft.packageId}>
              <option value="">{$t('admin.network.installations.select_package')}</option>
              {#each packages as pkg}
                <option value={pkg.id}>{pkg.name} - {packagePriceLabel(pkg)}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>{$t('admin.network.installations.billing_cycle')}</span>
            <select class="input" bind:value={draft.billingCycle}>
              <option value="monthly">{$t('admin.network.installations.monthly')}</option>
              <option value="yearly">{$t('admin.network.installations.yearly')}</option>
            </select>
          </label>
        </div>

        <div class="form-grid">
          <label><span>{$t('admin.network.installations.requested_install_date')}</span><input class="input" type="datetime-local" bind:value={draft.requestedInstallationDate} /></label>
          <label><span>{$t('admin.network.installations.order_notes')}</span><input class="input" bind:value={draft.notes} /></label>
        </div>
      </section>
    {/if}

    {#if step === 3}
      <section class="form-card section">
        <div class="section-header">
          <div>
            <h2>{$t('admin.network.installations.review_order')}</h2>
            <p>{$t('admin.network.installations.review_hint')}</p>
          </div>
        </div>

        <div class="summary-grid">
          <div class="summary-card summary-card-wide">
            <span>{$t('common.customer')}</span>
            <strong>{formatSelectedCustomer()}</strong>
          </div>
          <div class="summary-card">
            <span>{$t('admin.network.installations.address')}</span>
            <strong>{formatSelectedLocation()}</strong>
          </div>
          <div class="summary-card">
            <span>{$t('common.package')}</span>
            <strong>{selectedPackage?.name || '-'}</strong>
          </div>
          <div class="summary-card">
            <span>{$t('admin.network.installations.billing')}</span>
            <strong>{draft.billingCycle}</strong>
          </div>
          <div class="summary-card">
            <span>{$t('admin.network.installations.requested_install')}</span>
            <strong>{draft.requestedInstallationDate || '-'}</strong>
          </div>
          <div class="summary-card summary-card-wide">
            <span>{$t('common.notes')}</span>
            <strong>{draft.notes || '-'}</strong>
          </div>
        </div>

        <div class="review-band">
          <Icon name="shield-check" size={18} />
          <span>
            {$t('admin.network.installations.subscription_pending_note')}
          </span>
        </div>
      </section>
    {/if}

    <div class="actions footer-actions">
      <button class="btn btn-secondary" onclick={() => goto(customersPath)}>{$t('common.cancel')}</button>
      {#if step > 1}
        <button class="btn btn-secondary" onclick={prevStep}>{$t('common.back')}</button>
      {/if}
      {#if step < 3}
        <button class="btn btn-primary" onclick={nextStep}>
          {$t('admin.network.installations.continue')}
          <Icon name="chevron-right" size={16} />
        </button>
      {:else}
        <button class="btn btn-primary" onclick={submitOrder} disabled={submitting}>
          <Icon name="file-text" size={16} />
          {submitting ? $t('admin.network.installations.submitting') : $t('admin.network.installations.create_order_btn')}
        </button>
      {/if}
    </div>
  </div>
{/if}

<Modal show={showMapPicker} title={$t('admin.network.installations.pick_installation_point')} width="860px" onclose={closeMapPicker}>
  <div class="map-picker-shell">
    <div class="map-picker-help">
      {$t('admin.network.installations.map_picker_help')}
    </div>
    <div class="map-picker-cords">
      {#if pickerLat != null && pickerLng != null}
        <span class="mono-input">{pickerLat.toFixed(7)}, {pickerLng.toFixed(7)}</span>
      {/if}
    </div>
    <MapCanvasShell
      bind:mapEl={pickerMapHost}
      bind:viewMode={pickerViewMode}
      on:searchselect={onPickerSearchSelect}
      loading={pickerMapLoading}
      mapUnavailable={pickerMapUnavailable}
      mapErrorMessage={pickerMapErrorMessage}
      mapUnavailableTitle={$t('admin.network.installations.map_unavailable')}
      mapUnavailableSubtitle={$t('admin.network.installations.map_unavailable_subtitle')}
      height="min(58vh, 520px)"
    />
    <div class="actions">
      <button class="btn btn-secondary" type="button" onclick={closeMapPicker}>{$t('common.cancel')}</button>
      <button class="btn btn-primary" type="button" onclick={applyPickedCoordinates}>
        <Icon name="check" size={16} />
        {$t('admin.network.installations.use_this_point')}
      </button>
    </div>
  </div>
</Modal>

<style>
  .page-content {
    padding: 24px;
    max-width: 1400px;
    margin: 0 auto;
    display: grid;
    gap: 14px;
  }

  .hero-card,
  .form-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle, var(--border-color));
    border-radius: var(--radius-lg, 18px);
    box-shadow: var(--shadow-sm);
  }

  .hero-card {
    padding: 18px 20px;
  }

  .hero-copy {
    display: grid;
    gap: 4px;
  }

  h1 {
    margin: 0;
    font-size: 1.7rem;
    color: var(--text-primary);
  }

  .subtitle {
    margin: 0;
    color: var(--text-muted);
    max-width: 640px;
    line-height: 1.45;
  }

  .form-card {
    padding: 18px;
    display: grid;
    gap: 16px;
  }

  .section {
    display: grid;
    gap: 16px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: start;
  }

  .section-header h2 {
    margin: 0;
    font-size: 1.02rem;
    color: var(--text-primary);
  }

  .section-header p {
    margin: 3px 0 0;
    color: var(--text-muted);
    font-size: 0.84rem;
    line-height: 1.4;
  }

  .stepper {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }

  .stepper div {
    border-radius: 999px;
    border: 1px solid var(--border-subtle, var(--border-color));
    padding: 12px 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    text-align: center;
    color: var(--text-muted);
    background: color-mix(in srgb, var(--bg-surface) 72%, black);
  }

  .stepper div.active {
    background: linear-gradient(135deg, rgba(13, 148, 136, 0.92), rgba(15, 118, 110, 0.96));
    border-color: rgba(45, 212, 191, 0.45);
    color: white;
    box-shadow: 0 12px 30px rgba(13, 148, 136, 0.2);
  }

  .step-index {
    width: 24px;
    height: 24px;
    border-radius: 999px;
    display: grid;
    place-items: center;
    font-size: 0.78rem;
    font-weight: 800;
    border: 1px solid currentColor;
  }

  .segmented {
    display: inline-flex;
    gap: 8px;
    padding: 6px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle, var(--border-color));
    background: color-mix(in srgb, var(--bg-surface) 76%, black);
    width: fit-content;
    max-width: 100%;
    flex-wrap: wrap;
  }

  .mode-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    color: var(--text-muted);
    padding: 10px 16px;
    font-weight: 700;
    cursor: pointer;
  }

  .mode-btn.active-mode {
    border-color: rgba(45, 212, 191, 0.38);
    background: rgba(20, 184, 166, 0.16);
    color: rgb(204, 251, 241);
  }

  .mode-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px 14px;
  }

  .context-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  label {
    display: grid;
    gap: 7px;
    color: var(--text-primary);
    font-weight: 600;
  }

  label span {
    color: var(--text-primary);
  }

  .input {
    width: 100%;
    border: 1px solid var(--border-subtle, var(--border-color));
    border-radius: 12px;
    padding: 0.85rem 0.95rem;
    font: inherit;
    color: var(--text-primary);
    background: color-mix(in srgb, var(--bg-surface) 78%, black);
    outline: none;
  }

  .input:focus {
    border-color: rgba(45, 212, 191, 0.45);
    box-shadow: 0 0 0 3px rgba(20, 184, 166, 0.14);
  }

  textarea.input {
    resize: vertical;
    min-height: 112px;
  }

  .mono-input {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
  }

  .checkbox-row {
    display: flex !important;
    align-items: center;
    gap: 0.7rem;
    padding-top: 1.95rem;
    color: var(--text-primary);
  }

  .checkbox-row input {
    accent-color: rgb(20, 184, 166);
  }

  .inline-search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 2px 2px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-subtle, var(--border-color));
    background: color-mix(in srgb, var(--bg-surface) 78%, black);
  }

  .inline-search :global(svg) {
    color: var(--text-muted);
    flex: 0 0 auto;
  }

  .inline-search .input {
    border: none;
    background: transparent;
    box-shadow: none;
    padding-left: 0;
  }

  .phone-field {
    display: grid;
    grid-template-columns: minmax(180px, 220px) minmax(0, 1fr);
    gap: 10px;
    align-items: stretch;
  }

  .phone-prefix {
    min-width: 0;
  }

  .search-results {
    display: grid;
    gap: 10px;
  }

  .empty-state {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--text-muted);
    border: 1px dashed var(--border-subtle, var(--border-color));
    border-radius: 14px;
    padding: 12px 14px;
    background: color-mix(in srgb, var(--bg-surface) 70%, black);
  }

  .result-card {
    display: flex;
    justify-content: space-between;
    gap: 14px;
    align-items: center;
    text-align: left;
    border: 1px solid var(--border-subtle, var(--border-color));
    border-radius: 14px;
    padding: 14px 16px;
    background: var(--bg-primary);
    color: var(--text-primary);
    cursor: pointer;
  }

  .result-card.selected {
    border-color: rgba(45, 212, 191, 0.4);
    background: rgba(20, 184, 166, 0.1);
  }

  .result-main {
    display: grid;
    gap: 4px;
  }

  .result-main span {
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  .result-status {
    color: rgb(153, 246, 228);
    font-size: 0.78rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .context-card,
  .summary-card {
    border: 1px solid var(--border-subtle, var(--border-color));
    border-radius: 14px;
    background: var(--bg-primary);
    padding: 14px 16px;
    display: grid;
    gap: 6px;
  }

  .context-label,
  .summary-card span {
    color: var(--text-muted);
    font-size: 0.76rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }

  .context-card strong,
  .summary-card strong {
    color: var(--text-primary);
    line-height: 1.4;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .summary-card-wide {
    grid-column: 1 / -1;
  }

  .review-band {
    display: flex;
    gap: 10px;
    align-items: start;
    padding: 14px 16px;
    border-radius: 14px;
    border: 1px solid rgba(245, 158, 11, 0.28);
    background: rgba(245, 158, 11, 0.1);
    color: rgb(254, 240, 138);
  }

  .review-band strong {
    color: white;
  }

  .map-picked-box {
    border: 1px solid var(--border-subtle, var(--border-color));
    border-radius: 14px;
    padding: 0.95rem 1rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    background: var(--bg-primary);
  }

  .map-picked-title {
    color: var(--text-primary);
    font-weight: 700;
  }

  .map-picked-sub {
    margin-top: 0.25rem;
    color: var(--text-muted);
    font-size: 0.84rem;
    line-height: 1.4;
  }

  .map-picker-shell {
    display: grid;
    gap: 0.85rem;
  }

  .map-picker-help,
  .map-picker-cords {
    color: var(--text-muted);
  }

  .status-token {
    display: inline-flex;
    align-items: center;
    margin: 0 0.35rem;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.14);
    border: 1px solid rgba(255, 255, 255, 0.18);
    font-size: 0.8rem;
    letter-spacing: 0.02em;
  }

  .actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .footer-actions {
    padding-top: 4px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    font-weight: 800;
    cursor: pointer;
    text-decoration: none;
  }

  .btn-primary {
    background: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 65%, black);
    color: white;
  }

  .btn-secondary {
    background: transparent;
    color: var(--text-primary);
  }

  .btn-primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-primary) 84%, white);
    color: white;
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  @media (max-width: 768px) {
    .page-content {
      padding: 16px;
    }

    .hero-card {
      grid-template-columns: 1fr;
      display: grid;
    }

    .stepper,
    .form-grid,
    .context-grid,
    .summary-grid {
      grid-template-columns: 1fr;
    }

    .segmented,
    .inline-search,
    .actions,
    .map-picked-box {
      width: 100%;
      flex-direction: column;
    }

    .phone-field {
      grid-template-columns: 1fr;
    }

    .mode-btn,
    .btn {
      justify-content: center;
    }

    .result-card {
      flex-direction: column;
      align-items: start;
    }
  }
</style>
