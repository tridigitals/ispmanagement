<script lang="ts">
  /*
    Order instalasi baru v2 — gelombang 24c.

    Versi lama: (app)/admin/customers/orders/new/+page.svelte (1238 baris).
    Wizard 3 langkah identik + map picker. Modul colocated legacy dipakai
    ulang (customerSearchState, phoneField, orderWizardState); validasi +
    label harga kini helper murni orderWizardInsights (3 tes).
  */
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { onDestroy, onMount, tick } from 'svelte';
  import { get } from 'svelte/store';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import { api, type Customer, type CustomerListItem, type CustomerLocation, type IspPackage } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import { getVisibleInternetOrderPackages } from '$lib/utils/internetOrderPackages';
  import { getAdminCustomerNavigation } from '$lib/utils/adminCustomerNavigation';
  import { buildCountryOptions } from '$lib/utils/countryOptions';
  import { getCustomerSearchViewState } from '../../../../../../(app)/admin/customers/orders/new/customerSearchState';
  import {
    buildPhonePrefixOptions,
    composePhoneNumber,
    inferPhoneFieldState,
  } from '../../../../../../(app)/admin/customers/orders/new/phoneField';
  import {
    buildBackofficeInstallationOrderPayload,
    inferInitialCustomerMode,
    type OrderWizardDraft,
  } from '../../../../../../(app)/admin/customers/orders/new/orderWizardState';
  import {
    orderPackagePriceLabel,
    validateOrderStep1,
    validateOrderStep2,
  } from '$lib/utils/orderWizardInsights';
  import {
    AppShell,
    Button,
    Card,
    Field,
    PageHeader,
  } from '$lib/components/ds';
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
    customer: { name: '', email: '', phone: '', notes: '', is_active: true },
    locationMode: 'new',
    existingLocationId: '',
    location: {
      label: '', address_line1: '', address_line2: '', city: '', state: '',
      postal_code: '', country: 'ID', latitude: '', longitude: '', notes: '',
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
      if (!draft.packageId && packages.length > 0) draft.packageId = packages[0].id;
      if (prefilledCustomerId) {
        selectedCustomer = await api.customers.get(prefilledCustomerId);
        await loadLocations(prefilledCustomerId);
      }
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal memuat wizard order.');
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
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal cari pelanggan.');
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
    if (locations.length > 0 && !draft.existingLocationId) draft.existingLocationId = locations[0].id;
  }

  function nextStep() {
    if (step === 1) {
      const err = validateOrderStep1(draft);
      if (err) {
        toast.error(err);
        return;
      }
      step = 2;
      return;
    }
    if (step === 2) {
      const err = validateOrderStep2(draft);
      if (err) {
        toast.error(err);
        return;
      }
      step = 3;
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
      toast.success('Order instalasi dibuat.');
      if (canReadWorkOrders && result.work_order?.id) {
        goto(`/v2/admin/network/installations?work_order_id=${encodeURIComponent(result.work_order.id)}`);
        return;
      }
      goto(`/v2/admin/customers/${result.customer.id}`);
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal buat order.');
    } finally {
      submitting = false;
    }
  }

  function packagePriceLabel(pkg: IspPackage) {
    return orderPackagePriceLabel(Number(pkg.price_monthly || 0), Number(pkg.price_yearly || 0), draft.billingCycle);
  }

  function formatSelectedCustomer() {
    if (draft.customerMode === 'new') return draft.customer.name || '-';
    return selectedCustomer?.name || draft.existingCustomerId || '-';
  }

  function formatSelectedLocation() {
    if (draft.locationMode === 'new') return draft.location.label || draft.location.address_line1 || '-';
    return locations.find((l) => l.id === draft.existingLocationId)?.label || draft.existingLocationId || '-';
  }

  function parseCoordOrNull(value: string) {
    const raw = value.trim();
    if (!raw) return null;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : NaN;
  }

  async function getMaplibre() {
    if (!maplibrePromise) maplibrePromise = import('maplibre-gl');
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
    pickerMarker = new (pickerMap as any).libregl.Marker({ draggable: true }).setLngLat([lng, lat]).addTo(pickerMap);
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
  }

  async function openMapPicker() {
    const initialLat = parseCoordOrNull(draft.location.latitude);
    const initialLng = parseCoordOrNull(draft.location.longitude);
    const nextLat = typeof initialLat === 'number' && Number.isFinite(initialLat) ? initialLat : -6.2;
    const nextLng = typeof initialLng === 'number' && Number.isFinite(initialLng) ? initialLng : 106.816666;
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
                  ? [`https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=${pickerMapTilerKey}`]
                  : ['https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'],
                tileSize: 256,
                attribution: pickerMapTilerKey ? 'MapTiler' : 'Esri',
                maxzoom: pickerSatelliteMaxZoom,
              },
            },
            layers: [
              { id: 'order-picker-standard', type: 'raster', source: 'standard' },
              { id: 'order-picker-satellite', type: 'raster', source: 'satellite', layout: { visibility: 'none' } },
            ],
          },
          center: [nextLng, nextLat],
          zoom: 13,
          maxZoom: pickerStandardMaxZoom,
        });
        (pickerMap as any).libregl = libregl;
        pickerMap.addControl(new libregl.NavigationControl({ showCompass: true, showZoom: true }), 'top-right');
        pickerMap.addControl(new libregl.GeolocateControl({ trackUserLocation: false, showAccuracyCircle: true }), 'top-right');
        pickerMap.on('click', (event: any) => {
          const { lat, lng } = event.lngLat;
          setPickerPoint(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
        });
      } else {
        pickerMap.resize();
        pickerMap.setCenter([nextLng, nextLat]);
        pickerMap.setZoom(13);
      }
      syncPickerViewMode();
      setPickerPoint(nextLat, nextLng);
    } catch (e) {
      pickerMapUnavailable = true;
      pickerMapErrorMessage = extractApiErrorMessage(e) || 'Gagal buka peta.';
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
    pickerMap?.flyTo({ center: [lng, lat], zoom: Math.max(pickerMap.getZoom(), 15), duration: 480 });
  }

  function applyPickedCoordinates() {
    if (!Number.isFinite(pickerLat) || !Number.isFinite(pickerLng)) {
      toast.error('Pilih titik di peta dulu.');
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
<AppShell title="Order instalasi baru">
  <PageHeader
    title="Order instalasi baru"
    eyebrow="Pelanggan"
    desc="Wizard 3 langkah: pelanggan → alamat & layanan → review."
  >
    {#snippet actions()}
      <Button variant="ghost" href="/v2/admin/customers">Batal</Button>
    {/snippet}
  </PageHeader>

  {#if loading}
    <Card><p class="py-10 text-center text-sm text-ink-500">Memuat wizard…</p></Card>
  {:else}
    <ol class="mb-3 flex items-center gap-2 text-sm">
      <li class="flex items-center gap-1.5 {step === 1 ? 'font-semibold text-ink-900' : 'text-ink-500'}"><span class="flex h-6 w-6 items-center justify-center rounded-full text-xs {step === 1 ? 'bg-ink-900 text-white' : 'bg-ink-100'}">1</span> Pelanggan</li>
      <li class="text-ink-300">→</li>
      <li class="flex items-center gap-1.5 {step === 2 ? 'font-semibold text-ink-900' : 'text-ink-500'}"><span class="flex h-6 w-6 items-center justify-center rounded-full text-xs {step === 2 ? 'bg-ink-900 text-white' : 'bg-ink-100'}">2</span> Alamat & layanan</li>
      <li class="text-ink-300">→</li>
      <li class="flex items-center gap-1.5 {step === 3 ? 'font-semibold text-ink-900' : 'text-ink-500'}"><span class="flex h-6 w-6 items-center justify-center rounded-full text-xs {step === 3 ? 'bg-ink-900 text-white' : 'bg-ink-100'}">3</span> Review</li>
    </ol>

    {#if step === 1}
      <Card title="Konteks pelanggan">
        <div class="mb-3 flex gap-1 rounded-xl bg-ink-50 p-1">
          <button type="button" class="flex-1 rounded-lg px-3 py-2 text-sm {draft.customerMode === 'new' ? 'bg-white font-semibold shadow' : 'text-ink-500'}" onclick={() => (draft.customerMode = 'new')}>Pelanggan baru</button>
          <button type="button" class="flex-1 rounded-lg px-3 py-2 text-sm {draft.customerMode === 'existing' ? 'bg-white font-semibold shadow' : 'text-ink-500'}" onclick={() => (draft.customerMode = 'existing')}>Sudah ada</button>
        </div>
        {#if draft.customerMode === 'existing'}
          <div class="flex gap-2">
            <div class="flex-1">
              <Field id="ord-search" label="Cari pelanggan" placeholder="Nama / HP / email… (min 2 huruf)" value={customerSearch} onchange={(v) => { customerSearch = v; }} />
            </div>
            <div class="pt-6">
              <Button variant="ghost" onclick={() => void searchCustomers()} disabled={customerSearchLoading || customerSearch.trim().length < 2}>
                {customerSearchLoading ? 'Mencari…' : 'Cari'}
              </Button>
            </div>
          </div>
          <div class="mt-2 grid gap-2">
            {#if customerSearchView.kind !== 'results'}
              <p class="py-4 text-center text-sm text-ink-500">{customerSearchView.message}</p>
            {:else}
              {#each customerResults as customer}
                <button type="button" class="rounded-xl px-3 py-2.5 text-left ring-1 ring-ink-200 hover:bg-ink-50 {draft.existingCustomerId === customer.id ? 'ring-2 ring-ink-900' : ''}" onclick={() => void selectCustomer(customer.id)}>
                  <div class="font-medium">{customer.name}</div>
                  <div class="text-xs text-ink-500">{customer.phone || customer.email || 'Tanpa kontak'}</div>
                </button>
              {/each}
            {/if}
          </div>
        {:else}
          <div class="grid gap-2 sm:grid-cols-2">
            <Field stacked id="ord-name" label="Nama" value={draft.customer.name} onchange={(v) => (draft.customer.name = v)} />
            <Field stacked id="ord-email" label="Email" type="email" value={draft.customer.email} onchange={(v) => (draft.customer.email = v)} />
            <div class="flex gap-2">
              <div class="w-28 shrink-0">
                <Select2 bind:value={phonePrefix} options={phonePrefixOptions} width="100%" />
              </div>
              <div class="flex-1">
                <Field stacked id="ord-phone" label="No. HP" placeholder="8123456789" value={phoneLocalNumber} onchange={(v) => (phoneLocalNumber = v)} />
              </div>
            </div>
            <Field stacked id="ord-notes" label="Catatan" type="textarea" rows={2} value={draft.customer.notes} onchange={(v) => (draft.customer.notes = v)} />
          </div>
        {/if}
      </Card>
    {/if}

    {#if step === 2}
      <Card title="Alamat & layanan">
        <div class="mb-3 flex gap-1 rounded-xl bg-ink-50 p-1">
          <button type="button" class="flex-1 rounded-lg px-3 py-2 text-sm {draft.locationMode === 'existing' ? 'bg-white font-semibold shadow' : 'text-ink-500'}" disabled={draft.customerMode === 'new' && !draft.customer.name.trim()} onclick={() => (draft.locationMode = 'existing')}>Alamat tersimpan</button>
          <button type="button" class="flex-1 rounded-lg px-3 py-2 text-sm {draft.locationMode === 'new' ? 'bg-white font-semibold shadow' : 'text-ink-500'}" onclick={() => (draft.locationMode = 'new')}>Alamat baru</button>
        </div>
        {#if draft.locationMode === 'existing'}
          <Field stacked id="ord-loc" label="Alamat pelanggan" type="select" value={draft.existingLocationId} options={[{ value: '', label: 'Pilih alamat' }, ...locations.map((l) => ({ value: l.id, label: `${l.label} — ${l.address_line1 || 'tanpa alamat'}` }))]} onchange={(v) => (draft.existingLocationId = v)} />
        {:else}
          <div class="grid gap-2 sm:grid-cols-2">
            <Field stacked id="ord-llabel" label="Label lokasi" value={draft.location.label} onchange={(v) => (draft.location.label = v)} />
            <Field stacked id="ord-laddr1" label="Alamat baris 1" value={draft.location.address_line1} onchange={(v) => (draft.location.address_line1 = v)} />
            <Field stacked id="ord-laddr2" label="Alamat baris 2" value={draft.location.address_line2} onchange={(v) => (draft.location.address_line2 = v)} />
            <Field stacked id="ord-lcity" label="Kota" value={draft.location.city} onchange={(v) => (draft.location.city = v)} />
            <Field stacked id="ord-lstate" label="Provinsi" value={draft.location.state} onchange={(v) => (draft.location.state = v)} />
            <Field stacked id="ord-lpostal" label="Kode pos" value={draft.location.postal_code} onchange={(v) => (draft.location.postal_code = v)} />
          </div>
          <div class="mt-2 flex items-center justify-between rounded-xl bg-ink-50 px-3 py-2.5">
            <div class="text-sm"><div class="font-medium">Titik instalasi {draft.location.latitude && draft.location.longitude ? `(${draft.location.latitude}, ${draft.location.longitude})` : '(belum dipilih)'}</div><div class="text-xs text-ink-500">Opsional — teknisi pakai titik ini di lapangan.</div></div>
            <Button variant="ghost" onclick={() => void openMapPicker()}>Pilih di peta</Button>
          </div>
          <div class="mt-2">
            <Field stacked id="ord-lnotes" label="Catatan lokasi" type="textarea" rows={2} value={draft.location.notes} onchange={(v) => (draft.location.notes = v)} />
          </div>
        {/if}
        <div class="mt-3 grid gap-2 sm:grid-cols-2">
          <Field stacked id="ord-pkg" label="Paket" type="select" value={draft.packageId} options={[{ value: '', label: 'Pilih paket' }, ...packages.map((pkg) => ({ value: pkg.id, label: `${pkg.name} — ${packagePriceLabel(pkg)}` }))]} onchange={(v) => (draft.packageId = v)} />
          <Field stacked id="ord-cycle" label="Siklus billing" type="select" value={draft.billingCycle} options={[{ value: 'monthly', label: 'Bulanan' }, { value: 'yearly', label: 'Tahunan' }]} onchange={(v) => (draft.billingCycle = v as 'monthly' | 'yearly')} />
          <Field stacked id="ord-date" label="Tanggal instalasi" type="text" placeholder="2026-09-10 09:00" value={draft.requestedInstallationDate} onchange={(v) => (draft.requestedInstallationDate = v)} />
          <Field stacked id="ord-onotes" label="Catatan order" value={draft.notes} onchange={(v) => (draft.notes = v)} />
        </div>
      </Card>
    {/if}

    {#if step === 3}
      <Card title="Review order">
        <dl class="grid gap-3 text-sm sm:grid-cols-2">
          <div><dt class="text-xs text-ink-500">Pelanggan</dt><dd class="font-medium">{formatSelectedCustomer()}</dd></div>
          <div><dt class="text-xs text-ink-500">Alamat</dt><dd class="font-medium">{formatSelectedLocation()}</dd></div>
          <div><dt class="text-xs text-ink-500">Paket</dt><dd class="font-medium">{selectedPackage?.name || '-'}</dd></div>
          <div><dt class="text-xs text-ink-500">Billing</dt><dd class="font-medium">{draft.billingCycle}</dd></div>
          <div><dt class="text-xs text-ink-500">Tgl instalasi</dt><dd class="font-medium">{draft.requestedInstallationDate || '-'}</dd></div>
          <div><dt class="text-xs text-ink-500">Catatan</dt><dd class="font-medium">{draft.notes || '-'}</dd></div>
        </dl>
        <p class="mt-3 rounded-xl bg-amber-50 px-3 py-2.5 text-xs text-amber-800">Order dibuat sebagai pending — langganan aktif setelah instalasi selesai.</p>
      </Card>
    {/if}

    <div class="mt-3 flex justify-between gap-2">
      <Button variant="ghost" href="/v2/admin/customers">Batal</Button>
      <div class="flex gap-2">
        {#if step > 1}
          <Button variant="ghost" onclick={prevStep}>Kembali</Button>
        {/if}
        {#if step < 3}
          <Button variant="primary" onclick={nextStep}>Lanjut</Button>
        {:else}
          <Button variant="primary" onclick={() => void submitOrder()} disabled={submitting}>
            {submitting ? 'Menyimpan…' : 'Buat order'}
          </Button>
        {/if}
      </div>
    </div>
  {/if}
</AppShell>

<Modal bind:show={showMapPicker} title="Pilih titik instalasi" width="860px" onclose={closeMapPicker}>
  <div class="text-sm text-ink-500">Klik peta untuk menaruh pin, atau geser pin yang sudah ada.</div>
  {#if pickerLat != null && pickerLng != null}
    <p class="mt-1 font-mono text-xs">{pickerLat.toFixed(7)}, {pickerLng.toFixed(7)}</p>
  {/if}
  <div class="mt-2">
    <MapCanvasShell
      bind:mapEl={pickerMapHost}
      bind:viewMode={pickerViewMode}
      on:searchselect={onPickerSearchSelect}
      loading={pickerMapLoading}
      mapUnavailable={pickerMapUnavailable}
      mapErrorMessage={pickerMapErrorMessage}
      height="min(58vh, 520px)"
    />
  </div>
  <div class="mt-3 flex justify-end gap-2">
    <Button variant="ghost" onclick={closeMapPicker}>Batal</Button>
    <Button variant="primary" onclick={applyPickedCoordinates}>Pakai titik ini</Button>
  </div>
</Modal>
