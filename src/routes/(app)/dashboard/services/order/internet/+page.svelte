<script lang="ts">
  import { goto } from '$app/navigation';
  import { onDestroy, onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import { api, type CustomerLocation, type IspPackage } from '$lib/api/client';
  import { getVisibleInternetOrderPackages } from '$lib/utils/internetOrderPackages';
  import { appSettings } from '$lib/stores/settings';
  import { toast } from '$lib/stores/toast';

  type Step = 1 | 2 | 3;

  let loading = $state(true);
  let submitLoading = $state(false);
  let loadError = $state('');

  let locations = $state<CustomerLocation[]>([]);
  let basePackages = $state<IspPackage[]>([]);

  let step = $state<Step>(1);

  let draftLocationId = $state('');
  let draftPackageId = $state('');
  let draftBillingCycle = $state<'monthly' | 'yearly'>('monthly');

  let orderItems = $state<
    Array<{
      id: string;
      location_id: string;
      package_id: string;
      billing_cycle: 'monthly' | 'yearly';
    }>
  >([]);

  let showAddLocationModal = $state(false);
  let creatingLocation = $state(false);

  let newLocationLabel = $state('');
  let newLocationAddress = $state('');
  let newLocationCity = $state('');
  let newLocationState = $state('');
  let newLocationPostalCode = $state('');
  let newLocationCountry = $state('ID');
  let newLocationLatitude = $state('');
  let newLocationLongitude = $state('');
  let newLocationNotes = $state('');

  // ── Map picker state ──────────────────────────────────────────────
  // Replaces the old latitude/longitude text inputs with a click-to-pick
  // map. The text state above is kept as the data sink so the existing
  // save/lifecycle logic continues to work unchanged.
  let mapPickerContainer = $state<HTMLDivElement | null>(null);
  let mapPickerMounted = $state(false);
  let mapPickerError = $state('');
  let mapPickerDetecting = $state(false);
  let pickerMapInstance: any = null;
  let pickerMarkerInstance: any = null;
  let pickerMaplibreAny: any = null;

  function parseCoord(raw: string): number | null {
    const trimmed = (raw ?? '').trim();
    if (!trimmed) return null;
    const n = Number(trimmed);
    return Number.isFinite(n) ? n : null;
  }

  type TxValues = Record<string, string | number | boolean | Date | null | undefined>;

  function tx(key: string, fallback: string, values?: TxValues) {
    return get(t)(key, values ? { values } : undefined) || fallback;
  }

  onMount(() => {
    void loadData();
  });

  // ── Map picker lifecycle ──────────────────────────────────────────
  // Lazy-import maplibre-gl so the chunk only loads when the modal is
  // actually opened. Single OSM raster style (no API key required).
  async function ensureMapPickerReady() {
    if (pickerMapInstance) return;
    if (!mapPickerContainer) {
      // Wait one tick for the Modal to mount the DOM node.
      await new Promise((r) => setTimeout(r, 50));
      if (!mapPickerContainer) {
        mapPickerError = tx(
          'dashboard.internet_order.toasts.map_container_missing',
          'Peta tidak dapat dimuat (container hilang). Silakan tutup dan buka kembali.',
        );
        return;
      }
    }
    try {
      const maplibreMod = await import('maplibre-gl');
      await import('maplibre-gl/dist/maplibre-gl.css');
      const maplibre = (maplibreMod as any).default ?? maplibreMod;

      const initialLat = parseCoord(newLocationLatitude) ?? -2.5489;
      const initialLng = parseCoord(newLocationLongitude) ?? 118.0149;
      const hasMarker = parseCoord(newLocationLatitude) !== null && parseCoord(newLocationLongitude) !== null;

      const map = new maplibre.Map({
        container: mapPickerContainer,
        // Free OSM raster style — no token, no quota, works on any domain.
        style: {
          version: 8,
          sources: {
            'osm-raster': {
              type: 'raster',
              tiles: [
                'https://a.tile.openstreetmap.org/{z}/{x}/{y}.png',
                'https://b.tile.openstreetmap.org/{z}/{x}/{y}.png',
                'https://c.tile.openstreetmap.org/{z}/{x}/{y}.png',
              ],
              tileSize: 256,
              attribution: '© OpenStreetMap contributors',
            },
          },
          layers: [
            { id: 'osm-raster-layer', type: 'raster', source: 'osm-raster' },
          ],
        },
        center: [initialLng, initialLat],
        zoom: hasMarker ? 12 : 5,
      });

      map.addControl(new maplibre.NavigationControl({ showCompass: false }), 'top-right');
      map.addControl(new maplibre.ScaleControl({ unit: 'metric' }), 'bottom-left');

      const syncFromLngLat = (lng: number, lat: number) => {
        newLocationLatitude = lat.toFixed(6);
        newLocationLongitude = lng.toFixed(6);
      };

      // Seed marker if we already have coords in state.
      if (hasMarker) {
        pickerMarkerInstance = new maplibre.Marker({ draggable: true, color: '#d33' })
          .setLngLat([initialLng, initialLat])
          .addTo(map);
        pickerMarkerInstance.on('dragend', () => {
          const ll = pickerMarkerInstance.getLngLat();
          syncFromLngLat(ll.lng, ll.lat);
        });
      }

      // Click anywhere on the map to drop/reposition the marker.
      map.on('click', (ev: any) => {
        const lng = ev.lngLat.lng;
        const lat = ev.lngLat.lat;
        syncFromLngLat(lng, lat);
        if (pickerMarkerInstance) {
          pickerMarkerInstance.setLngLat([lng, lat]);
        } else {
          pickerMarkerInstance = new maplibre.Marker({ draggable: true, color: '#d33' })
            .setLngLat([lng, lat])
            .addTo(map);
          pickerMarkerInstance.on('dragend', () => {
            const ll = pickerMarkerInstance.getLngLat();
            syncFromLngLat(ll.lng, ll.lat);
          });
        }
      });

      pickerMapInstance = map;
      pickerMaplibreAny = maplibre;
      mapPickerMounted = true;
      mapPickerError = '';
    } catch (err) {
      console.error('[internet-order] failed to init map picker', err);
      mapPickerError = tx(
        'dashboard.internet_order.toasts.map_load_failed',
        'Peta gagal dimuat. Silakan cek koneksi internet atau coba lagi.',
      );
    }
  }

  function teardownMapPicker() {
    try {
      if (pickerMarkerInstance) {
        pickerMarkerInstance.remove();
      }
    } catch {
      /* ignore */
    }
    try {
      if (pickerMapInstance) {
        pickerMapInstance.remove();
      }
    } catch {
      /* ignore */
    }
    pickerMarkerInstance = null;
    pickerMapInstance = null;
    pickerMaplibreAny = null;
    mapPickerMounted = false;
  }

  $effect(() => {
    if (showAddLocationModal) {
      void ensureMapPickerReady();
    } else {
      teardownMapPicker();
    }
  });

  onDestroy(() => {
    teardownMapPicker();
  });

  async function detectMyLocation() {
    if (typeof navigator === 'undefined' || !navigator.geolocation) {
      toast.error(
        tx(
          'dashboard.internet_order.toasts.geolocation_unsupported',
          'Browser Anda tidak mendukung deteksi lokasi otomatis.',
        ),
      );
      return;
    }
    mapPickerDetecting = true;
    try {
      const position = await new Promise<GeolocationPosition>((resolve, reject) => {
        navigator.geolocation.getCurrentPosition(resolve, reject, {
          enableHighAccuracy: true,
          timeout: 8000,
          maximumAge: 60_000,
        });
      });
      const lat = position.coords.latitude;
      const lng = position.coords.longitude;
      newLocationLatitude = lat.toFixed(6);
      newLocationLongitude = lng.toFixed(6);
      if (pickerMapInstance && pickerMaplibreAny) {
        pickerMapInstance.flyTo({ center: [lng, lat], zoom: 14 });
        if (pickerMarkerInstance) {
          pickerMarkerInstance.setLngLat([lng, lat]);
        } else {
          pickerMarkerInstance = new pickerMaplibreAny.Marker({ draggable: true, color: '#d33' })
            .setLngLat([lng, lat])
            .addTo(pickerMapInstance);
          pickerMarkerInstance.on('dragend', () => {
            const ll = pickerMarkerInstance.getLngLat();
            newLocationLatitude = ll.lat.toFixed(6);
            newLocationLongitude = ll.lng.toFixed(6);
          });
        }
      }
    } catch (err) {
      toast.error(
        tx(
          'dashboard.internet_order.toasts.geolocation_failed',
          'Tidak dapat mendeteksi lokasi. Pastikan izin lokasi diaktifkan.',
        ),
      );
    } finally {
      mapPickerDetecting = false;
    }
  }

  const selectedLocation = $derived.by(
    () => locations.find((location) => location.id === draftLocationId) || null,
  );

  const packages = $derived.by(() => getVisibleInternetOrderPackages(basePackages));

  $effect(() => {
    const list = packages;
    if (loading || list.length === 0) return;
    if (!list.some((pkg) => pkg.id === draftPackageId)) {
      draftPackageId = list[0].id;
    }
  });

  const draftPackage = $derived.by(() => getPackageById(draftPackageId));
  const draftAmount = $derived.by(() => {
    const pkg = draftPackage;
    if (!pkg) return 0;
    if (draftBillingCycle === 'yearly' && hasYearlyPrice(pkg)) return Number(pkg.price_yearly || 0);
    return Number(pkg.price_monthly || 0);
  });

  const orderTotalAmount = $derived.by(() =>
    orderItems.reduce((sum, item) => sum + getOrderItemAmount(item), 0),
  );

  $effect(() => {
    const pkg = draftPackage;
    if (!pkg) return;
    if (draftBillingCycle === 'yearly' && !hasYearlyPrice(pkg)) {
      draftBillingCycle = 'monthly';
    }
  });

  async function loadData() {
    loading = true;
    loadError = '';
    try {
      const [myLocations, myPackages] = await Promise.all([
        api.customers.portal.myLocations(),
        api.customers.portal.myPackages(),
      ]);

      locations = myLocations || [];
      basePackages = (myPackages || []).filter((pkg) => pkg.is_active);

      if (!draftLocationId && locations.length > 0) draftLocationId = locations[0].id;
      if (!draftPackageId && basePackages.length > 0) draftPackageId = basePackages[0].id;
    } catch (e: any) {
      loadError = e?.message || String(e);
      toast.error(tx('dashboard.internet_order.toasts.load_failed', 'Gagal memuat katalog layanan internet'));
    } finally {
      loading = false;
    }
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

  function hasYearlyPrice(pkg: IspPackage) {
    return Number(pkg.price_yearly || 0) > 0;
  }

  function getPackageById(id: string) {
    return basePackages.find((pkg) => pkg.id === id) || null;
  }

  function billingCycleLabel(cycle: 'monthly' | 'yearly' | string) {
    return cycle === 'yearly'
      ? tx('dashboard.internet_order.cycles.yearly', 'Tahunan')
      : tx('dashboard.internet_order.cycles.monthly', 'Bulanan');
  }

  function locationLabel(locationId: string) {
    return locations.find((l) => l.id === locationId)?.label || locationId;
  }

  function getOrderItemAmount(item: { package_id: string; billing_cycle: 'monthly' | 'yearly' }) {
    const pkg = getPackageById(item.package_id);
    if (!pkg) return 0;
    if (item.billing_cycle === 'yearly' && hasYearlyPrice(pkg)) {
      return Number(pkg.price_yearly || 0);
    }
    return Number(pkg.price_monthly || 0);
  }

  function checkoutEligibilityError(
    pkg: IspPackage,
    cycle: 'monthly' | 'yearly',
    locationId?: string,
  ): string | null {
    const targetLocationId = locationId || draftLocationId;
    if (!targetLocationId) {
      return tx('dashboard.internet_order.toasts.select_location_first', 'Pilih alamat terlebih dahulu');
    }

    if (cycle === 'yearly' && !hasYearlyPrice(pkg)) {
      return tx('dashboard.internet_order.toasts.yearly_unavailable', 'Paket ini belum mendukung tagihan tahunan');
    }

    return null;
  }

  function moveToPackageStep() {
    if (!draftLocationId) {
      toast.error(tx('dashboard.internet_order.toasts.select_location_first', 'Pilih alamat terlebih dahulu'));
      return;
    }
    step = 2;
  }

  function moveBackToAddressStep() {
    step = 1;
  }

  function orderNowFromPackage() {
    const pkg = getPackageById(draftPackageId);
    if (!pkg) {
      toast.error(tx('dashboard.internet_order.toasts.invalid_package', 'Paket tidak valid'));
      return;
    }

    const eligibilityError = checkoutEligibilityError(pkg, draftBillingCycle, draftLocationId);
    if (eligibilityError) {
      toast.error(eligibilityError);
      return;
    }

    orderItems = [
      ...orderItems,
      {
        id: crypto.randomUUID(),
        location_id: draftLocationId,
        package_id: draftPackageId,
        billing_cycle: draftBillingCycle,
      },
    ];

    toast.success(tx('dashboard.internet_order.toasts.added_to_order', 'Item berhasil ditambahkan ke pesanan'));
    step = 3;
  }

  function removeOrderItem(id: string) {
    orderItems = orderItems.filter((item) => item.id !== id);
  }

  function addMoreFromStep3() {
    step = 1;
  }

  async function submitBulkOrder() {
    if (submitLoading || orderItems.length === 0) return;

    for (const item of orderItems) {
      const pkg = getPackageById(item.package_id);
      if (!pkg) {
        toast.error(
          tx('dashboard.internet_order.toasts.invalid_package_in_order', 'Ada paket tidak valid di daftar pesanan'),
        );
        return;
      }
      const eligibilityError = checkoutEligibilityError(pkg, item.billing_cycle, item.location_id);
      if (eligibilityError) {
        toast.error(eligibilityError);
        return;
      }
    }

    submitLoading = true;
    const created: Array<{ id: string; work_order_id?: string | null }> = [];
    try {
      for (const item of orderItems) {
        const res = await api.customers.portal.orderRequest({
          location_id: item.location_id,
          package_id: item.package_id,
          billing_cycle: item.billing_cycle,
        });
        if (res?.subscription?.id) {
          created.push({ id: res.subscription.id, work_order_id: res.work_order?.id });
        }
      }

      toast.success(
        created.length === 1
          ? tx('dashboard.internet_order.toasts.request_submitted_single', 'Permintaan instalasi berhasil dikirim ({id})', {
              id: created[0].work_order_id || created[0].id,
            })
          : tx(
              'dashboard.internet_order.toasts.request_submitted_multi',
              '{count} permintaan instalasi berhasil dikirim',
              { count: created.length },
            ),
      );

      orderItems = [];
      await goto('/dashboard/services');
    } catch (e: any) {
      toast.error(
        e?.message ||
          tx('dashboard.internet_order.toasts.request_submit_failed', 'Gagal mengirim permintaan instalasi'),
      );
    } finally {
      submitLoading = false;
    }
  }

  function openAddLocationModal() {
    newLocationLabel = '';
    newLocationAddress = '';
    newLocationCity = '';
    newLocationState = '';
    newLocationPostalCode = '';
    newLocationCountry = 'ID';
    newLocationLatitude = '';
    newLocationLongitude = '';
    newLocationNotes = '';
    showAddLocationModal = true;
  }

  async function saveMyLocation() {
    if (creatingLocation || !newLocationLabel.trim()) return;
    const latRaw = newLocationLatitude.trim();
    const lngRaw = newLocationLongitude.trim();
    const parsedLat = latRaw ? Number(latRaw) : NaN;
    const parsedLng = lngRaw ? Number(lngRaw) : NaN;

    if (latRaw && (Number.isNaN(parsedLat) || parsedLat < -90 || parsedLat > 90)) {
      toast.error(tx('dashboard.internet_order.toasts.latitude_range', 'Latitude harus di antara -90 hingga 90'));
      return;
    }
    if (lngRaw && (Number.isNaN(parsedLng) || parsedLng < -180 || parsedLng > 180)) {
      toast.error(tx('dashboard.internet_order.toasts.longitude_range', 'Longitude harus di antara -180 hingga 180'));
      return;
    }

    const latitude = latRaw ? parsedLat : null;
    const longitude = lngRaw ? parsedLng : null;

    creatingLocation = true;
    try {
      await api.customers.portal.createMyLocation({
        label: newLocationLabel.trim(),
        address_line1: newLocationAddress.trim() || null,
        city: newLocationCity.trim() || null,
        state: newLocationState.trim() || null,
        postal_code: newLocationPostalCode.trim() || null,
        country: newLocationCountry.trim() || null,
        latitude,
        longitude,
        notes: newLocationNotes.trim() || null,
      });

      toast.success(tx('dashboard.internet_order.toasts.location_saved', 'Lokasi berhasil disimpan'));
      showAddLocationModal = false;
      await loadData();
      if (locations.length > 0) {
        draftLocationId = locations[0].id;
      }
    } catch (e: any) {
      toast.error(e?.message || tx('dashboard.internet_order.toasts.location_create_failed', 'Gagal menambahkan lokasi'));
    } finally {
      creatingLocation = false;
    }
  }
</script>

<div class="internet-order-page fade-in">
  <!-- Page Header -->
  <section class="page-header">
    <div class="kicker">
      <span class="kicker-dot"></span>
      {$t('dashboard.internet_order.hero.title')}
    </div>
    <h1>{$t('dashboard.internet_order.hero.title')}</h1>
    <p>{$t('dashboard.internet_order.hero.subtitle')}</p>
    <div class="hero-actions" style="margin-top:0.6rem">
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/services/order')}>
        <Icon name="arrow-left" size={15} />
        {$t('dashboard.internet_order.actions.service_types')}
      </button>
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/invoices')}>
        <Icon name="file-text" size={15} />
        {$t('dashboard.internet_order.actions.billing_invoices')}
      </button>
      <button class="btn btn-secondary" type="button" onclick={loadData} disabled={loading}>
        <Icon name="refresh-cw" size={15} />
        {$t('dashboard.internet_order.actions.refresh')}
      </button>
    </div>
  </section>

  {#if loadError}
    <section class="alert alert-error">{loadError}</section>
  {/if}

  <!-- Two-column layout: stepper sidebar + content -->
  <div class="order-layout">
    <!--Vertical Stepper Sidebar -->
    <nav class="stepper-sidebar">
      <button class="stepper-step" class:active={step === 1} type="button" onclick={() => (step = 1)}>
        <span class="step-bullet">1</span>
        <div class="step-label">
          <span class="step-title">{$t('dashboard.internet_order.steps.address')}</span>
          <span class="step-subtitle">{$t('dashboard.internet_order.stage.address.subtitle')}</span>
        </div>
      </button>
      <div class="stepper-connector" class:filled={step >= 2}></div>
      <button class="stepper-step" class:active={step === 2} type="button" onclick={moveToPackageStep} disabled={!draftLocationId}>
        <span class="step-bullet">2</span>
        <div class="step-label">
          <span class="step-title">{$t('dashboard.internet_order.steps.package')}</span>
          <span class="step-subtitle">{$t('dashboard.internet_order.stage.package.title')}</span>
        </div>
      </button>
      <div class="stepper-connector" class:filled={step >= 3}></div>
      <button class="stepper-step" class:active={step === 3} type="button" onclick={() => orderItems.length > 0 && (step = 3)} disabled={orderItems.length === 0}>
        <span class="step-bullet">3</span>
        <div class="step-label">
          <span class="step-title">{$t('dashboard.internet_order.steps.review')}</span>
          <span class="step-subtitle">{$t('dashboard.internet_order.stage.review.subtitle')}</span>
        </div>
      </button>
    </nav>

    <!-- Main Content Area -->
    <div class="order-content">
      {#if step === 1}
        <!-- Step 1: Pilih Alamat -->
        <section class="stage-card">
          <div class="stage-head">
            <div>
              <h3>{$t('dashboard.internet_order.stage.address.title')}</h3>
              <p>{$t('dashboard.internet_order.stage.address.subtitle')}</p>
            </div>
            <button class="btn btn-secondary" type="button" onclick={openAddLocationModal}>
              <Icon name="map-pin" size={15} />
              {$t('dashboard.internet_order.actions.add_location')}
            </button>
          </div>

          {#if !loading && locations.length === 0}
            <div class="empty-state">
              <Icon name="map-pin" size={18} />
              <div>
                <h4>{$t('dashboard.internet_order.empty.no_locations_title')}</h4>
                <p>{$t('dashboard.internet_order.empty.no_locations_subtitle')}</p>
              </div>
            </div>
          {:else}
            <div class="addr-grid">
              {#each locations as location (location.id)}
                <button
                  class="addr-option"
                  class:selected={draftLocationId === location.id}
                  type="button"
                  onclick={() => (draftLocationId = location.id)}
                >
                  <div class="addr-option-head">
                    <Icon name="map-pin" size={16} />
                    <span class="addr-option-label">{location.label}</span>
                    {#if draftLocationId === location.id}
                      <span class="pill pill-success">{$t('dashboard.internet_order.badges.selected')}</span>
                    {/if}
                  </div>
                  {#if location.address_line1}
                    <p class="addr-option-detail">{location.address_line1}</p>
                  {/if}
                  {#if location.city || location.state}
                    <p class="addr-option-meta">
                      {[location.city, location.state].filter(Boolean).join(', ')}
                      {#if location.postal_code} {location.postal_code}{/if}
                    </p>
                  {/if}
                </button>
              {/each}
            </div>

            <div class="stage-nav">
              <span></span>
              <button class="btn btn-primary" type="button" onclick={moveToPackageStep} disabled={!draftLocationId}>
                {$t('dashboard.internet_order.actions.next_choose_package')}
                <Icon name="arrow-right" size={15} />
              </button>
            </div>
          {/if}
        </section>

      {:else if step === 2}
        <!-- Step 2: Pilih Paket -->
        <section class="stage-card">
          <div class="stage-head">
            <div>
              <h3>{$t('dashboard.internet_order.stage.package.title')}</h3>
              <p>
                {$t('dashboard.internet_order.labels.location')}: <strong>{selectedLocation?.label || '-'}</strong>
              </p>
            </div>
          </div>

          <!-- Billing cycle toggle -->
          <div class="cycle-toolbar">
            <div class="cycle-pills">
              <button
                class="cycle-pill"
                class:active={draftBillingCycle === 'monthly'}
                type="button"
                onclick={() => (draftBillingCycle = 'monthly')}
              >
                {$t('dashboard.internet_order.cycles.monthly')}
              </button>
              <button
                class="cycle-pill"
                class:active={draftBillingCycle === 'yearly'}
                type="button"
                onclick={() => (draftBillingCycle = 'yearly')}
                disabled={!draftPackage || !hasYearlyPrice(draftPackage)}
              >
                {$t('dashboard.internet_order.cycles.yearly')}
              </button>
            </div>
          </div>

          {#if loading}
            <p class="status-note">{$t('dashboard.internet_order.status.loading_packages')}</p>
          {:else if packages.length === 0}
            <div class="empty-state">
              <Icon name="package" size={18} />
              <div>
                <h4>{$t('dashboard.internet_order.empty.no_packages_title')}</h4>
                <p>{$t('dashboard.internet_order.empty.no_packages_subtitle')}</p>
              </div>
            </div>
          {:else}
            <div class="pkg-grid">
              {#each packages as pkg (pkg.id)}
                <button
                  class="pkg-option"
                  class:selected={draftPackageId === pkg.id}
                  type="button"
                  onclick={() => (draftPackageId = pkg.id)}
                >
                  <div class="pkg-option-head">
                    <h4>{pkg.name}</h4>
                    {#if draftPackageId === pkg.id}
                      <span class="pill pill-success">{$t('dashboard.internet_order.badges.selected')}</span>
                    {/if}
                  </div>
                  {#if pkg.description}
                    <p class="pkg-option-desc">{pkg.description}</p>
                  {/if}
                  <div class="pkg-option-price">
                    <strong>{formatCurrency(Number(pkg.price_monthly || 0))}</strong>
                    <span>/{$t('dashboard.internet_order.cycles.monthly')}</span>
                  </div>
                  {#if hasYearlyPrice(pkg)}
                    <div class="pkg-option-yearly">
                      {formatCurrency(Number(pkg.price_yearly || 0))} {$t('dashboard.internet_order.labels.yearly_available')}
                    </div>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}

          <div class="stage-nav">
            <button class="btn btn-ghost" type="button" onclick={moveBackToAddressStep}>
              <Icon name="arrow-left" size={15} />
              {$t('dashboard.internet_order.actions.back_to_address')}
            </button>
            <button class="btn btn-primary" type="button" onclick={orderNowFromPackage} disabled={!draftPackageId}>
              {$t('dashboard.internet_order.actions.order_now')}
              <Icon name="arrow-right" size={15} />
            </button>
          </div>
        </section>

      {:else if step === 3}
        <!-- Step 3: Review & Konfirmasi -->
        <section class="stage-card">
          <div class="stage-head">
            <div>
              <h3>{$t('dashboard.internet_order.stage.review.title')}</h3>
              <p>{$t('dashboard.internet_order.stage.review.subtitle')}</p>
            </div>
          </div>

          {#if orderItems.length === 0}
            <div class="empty-state">
              <Icon name="clipboard-list" size={18} />
              <div>
                <h4>{$t('dashboard.internet_order.empty.no_order_items_title')}</h4>
                <p>{$t('dashboard.internet_order.empty.no_order_items_subtitle')}</p>
              </div>
            </div>
          {:else}
            <!-- Summary card -->
            <div class="summary-card">
              {#each orderItems as item, index (item.id)}
                {@const pkg = getPackageById(item.package_id)}
                <div class="summary-row">
                  <span class="summary-label">
                    <span class="queue-index" style="display:inline-flex;margin-right:.45rem;width:20px;height:20px;font-size:.68rem">{index + 1}</span>
                    {pkg?.name || item.package_id}
                  </span>
                  <span class="summary-value">{formatCurrency(getOrderItemAmount(item))}</span>
                </div>
                <div class="summary-row" style="font-size:.82rem;color:var(--text-secondary)">
                  <span>{locationLabel(item.location_id)} · {billingCycleLabel(item.billing_cycle)}</span>
                  <button class="btn btn-ghost" style="padding:.25rem .5rem;font-size:.75rem" type="button" onclick={() => removeOrderItem(item.id)}>
                    <Icon name="trash-2" size={12} />
                  </button>
                </div>
              {/each}
              <div class="summary-row" style="border-top:1px dashed var(--border-color);margin-top:.4rem;padding-top:.6rem">
                <span class="summary-label" style="font-weight:700">{$t('dashboard.internet_order.labels.total_order_amount')}</span>
                <span class="summary-value" style="font-size:1.15rem;color:var(--accent-primary)">{formatCurrency(orderTotalAmount)}</span>
              </div>
            </div>
          {/if}

          <div class="stage-nav">
            <div>
              <button class="btn btn-secondary" type="button" onclick={addMoreFromStep3}>
                <Icon name="plus" size={15} />
                {$t('dashboard.internet_order.actions.add_more')}
              </button>
            </div>
            <button
              class="btn btn-primary"
              type="button"
              onclick={submitBulkOrder}
              disabled={submitLoading || orderItems.length === 0}
            >
              {#if submitLoading}
                <Icon name="refresh-cw" size={16} />
                {$t('dashboard.internet_order.status.processing')}
              {:else}
                <Icon name="check-circle" size={16} />
                {$t('dashboard.internet_order.actions.submit_installation_request')}
              {/if}
            </button>
          </div>
        </section>
      {/if}
    </div>
  </div>
</div>

<Modal
  show={showAddLocationModal}
  title={$t('dashboard.internet_order.modal.add_location_title')}
  onclose={() => {
    if (!creatingLocation) showAddLocationModal = false;
  }}
>
  <div class="location-form">
    <label class="form-field">
      <span>{$t('dashboard.internet_order.modal.fields.label')}</span>
      <input class="input" bind:value={newLocationLabel} placeholder={$t('dashboard.internet_order.modal.placeholders.label')} />
    </label>
    <label class="form-field">
      <span>{$t('dashboard.internet_order.modal.fields.address')}</span>
      <input class="input" bind:value={newLocationAddress} placeholder={$t('dashboard.internet_order.modal.placeholders.address')} />
    </label>
    <div class="location-grid-2">
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.city')}</span>
        <input class="input" bind:value={newLocationCity} />
      </label>
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.state')}</span>
        <input class="input" bind:value={newLocationState} />
      </label>
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.postal_code')}</span>
        <input class="input" bind:value={newLocationPostalCode} />
      </label>
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.country')}</span>
        <input class="input" bind:value={newLocationCountry} />
      </label>
    </div>
    <label class="form-field">
      <span>{$t('dashboard.internet_order.modal.fields.notes')}</span>
      <textarea class="input textarea" bind:value={newLocationNotes} rows="3"></textarea>
    </label>

    <div class="map-picker">
      <div class="map-picker-header">
        <span class="form-field-label">
          {$t('dashboard.internet_order.modal.fields.coordinates_picker') ||
            'Pilih lokasi pada peta (klik atau geser marker)'}
        </span>
        <div class="map-picker-actions">
          <button
            type="button"
            class="btn btn-secondary btn-sm"
            onclick={detectMyLocation}
            disabled={mapPickerDetecting}
          >
            <Icon
              name={mapPickerDetecting ? 'refresh-cw' : 'crosshair'}
              size={14}
            />
            {$t('dashboard.internet_order.actions.use_my_location') ||
              'Gunakan lokasi saya'}
          </button>
        </div>
      </div>
      <div
        class="map-picker-canvas"
        bind:this={mapPickerContainer}
        role="application"
        aria-label="Map picker"
      ></div>
      {#if mapPickerError}
        <p class="map-picker-error">{mapPickerError}</p>
      {/if}
      <div class="map-picker-coords">
        <span>
          <strong>Lat:</strong>
          {newLocationLatitude || '—'}
        </span>
        <span>
          <strong>Lng:</strong>
          {newLocationLongitude || '—'}
        </span>
        {#if !mapPickerMounted && !mapPickerError}
          <span class="map-picker-status">
            <Icon name="refresh-cw" size={12} />
            Memuat peta…
          </span>
        {/if}
      </div>
    </div>
    <div class="checkout-actions">
      <button
        class="btn btn-secondary"
        onclick={() => (showAddLocationModal = false)}
        disabled={creatingLocation}
      >
        {$t('dashboard.internet_order.actions.cancel')}
      </button>
      <button class="btn btn-primary" onclick={saveMyLocation} disabled={creatingLocation || !newLocationLabel.trim()}>
        {#if creatingLocation}
          <Icon name="refresh-cw" size={16} />
          {$t('dashboard.internet_order.status.saving')}
        {:else}
          <Icon name="save" size={16} />
          {$t('dashboard.internet_order.actions.save')}
        {/if}
      </button>
    </div>
  </div>
</Modal>

<style>
  .internet-order-page {
    max-width: 1240px;
    margin: 0 auto;
    padding: clamp(1rem, 2.2vw, 1.8rem);
    display: grid;
    gap: 0.9rem;
  }

  /* ── page header / kicker ── */
  .page-header h1 { margin: 0.15rem 0 0.25rem; }
  .page-header p { color: var(--text-secondary); font-size: 0.88rem; }
  .hero-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  /* ── two-column layout ── */
  .order-layout {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    gap: 1.2rem;
    align-items: start;
  }

  /* ── vertical stepper ── */
  .stepper-sidebar {
    display: flex;
    flex-direction: column;
    gap: 0;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-surface);
    padding: 0.8rem;
    position: sticky;
    top: 0.8rem;
  }
  .stepper-step {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.55rem 0.45rem;
    border: none;
    background: transparent;
    cursor: pointer;
    border-radius: 10px;
    text-align: left;
    width: 100%;
    transition: background 0.15s;
  }
  .stepper-step:disabled { opacity: 0.45; cursor: not-allowed; }
  .stepper-step.active { background: color-mix(in srgb, var(--accent-primary) 8%, transparent); }
  .step-bullet {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid var(--border-color);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.78rem;
    font-weight: 700;
    flex-shrink: 0;
    color: var(--text-secondary);
    background: var(--bg-secondary);
  }
  .stepper-step.active .step-bullet {
    border-color: var(--accent-primary);
    background: var(--accent-primary);
    color: var(--text-on-primary, #fff);
  }
  .step-label { display: grid; gap: 0.08rem; }
  .step-title { font-size: 0.85rem; font-weight: 650; line-height: 1.2; }
  .stepper-step.active .step-title { color: var(--accent-primary); }
  .step-subtitle {
    font-size: 0.72rem;
    color: var(--text-secondary);
    line-height: 1.25;
  }
  .stepper-connector {
    width: 2px;
    height: 18px;
    background: var(--border-color);
    margin-left: 1rem;
  }
  .stepper-connector.filled { background: var(--accent-primary); }

  /* ── content area ── */
  .order-content { min-width: 0; }

  /* ── stage card (no border, no padding — just gap) ── */
  .stage-card { display: grid; gap: 0.8rem; }

  .stage-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .stage-head h3 { margin: 0; font-size: 1.03rem; }
  .stage-head p { margin: 0.25rem 0 0; color: var(--text-secondary); font-size: 0.84rem; line-height: 1.45; }

  /* ── navigation row ── */
  .stage-nav {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
    border-top: 1px dashed var(--border-color);
    padding-top: 0.72rem;
  }

  /* ── address grid ── */
  .addr-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 0.65rem;
  }

  /* addr-option & pkg-option share base (selected state via global .selected) */
  .addr-option,
  .pkg-option {
    border: 2px solid var(--border-color);
    border-radius: 12px;
    padding: 0.85rem;
    background: var(--bg-surface);
    cursor: pointer;
    text-align: left;
    transition: border-color 0.2s, background 0.2s;
    display: grid;
    gap: 0.4rem;
  }
  .addr-option:hover,
  .pkg-option:hover { border-color: color-mix(in srgb, var(--accent-primary) 42%, var(--border-color)); }

  .addr-option-head,
  .pkg-option-head {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }
  .addr-option-label { font-weight: 650; font-size: 0.92rem; }
  .addr-option-detail { margin: 0; color: var(--text-primary); font-size: 0.83rem; line-height: 1.35; }
  .addr-option-meta { margin: 0; color: var(--text-secondary); font-size: 0.78rem; }

  .pkg-option-desc { margin: 0; color: var(--text-secondary); font-size: 0.8rem; line-height: 1.3; }
  .pkg-option-price { display: flex; align-items: baseline; gap: 0.3rem; }
  .pkg-option-price strong { font-size: 1.1rem; }
  .pkg-option-price span { color: var(--text-secondary); font-size: 0.78rem; }
  .pkg-option-yearly { color: var(--text-secondary); font-size: 0.79rem; }

  .pkg-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 0.7rem;
  }

  /* ── cycle toolbar ── */
  .cycle-toolbar {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-secondary) 72%, transparent);
    padding: 0.6rem 0.72rem;
  }
  .cycle-pills { display: inline-flex; gap: 0.5rem; align-items: center; }
  .cycle-pill {
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 0.4rem 0.75rem;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .cycle-pill:disabled { opacity: 0.5; cursor: not-allowed; }
  .cycle-pill.active {
    border-color: color-mix(in srgb, var(--accent-primary) 55%, var(--border-color));
    background: color-mix(in srgb, var(--accent-primary) 14%, transparent);
    color: var(--accent-primary);
    font-weight: 700;
  }

  /* ── misc ── */
  .empty-state {
    border: 1px dashed var(--border-color);
    border-radius: 10px;
    padding: 0.72rem;
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
  }
  .empty-state h4 { margin: 0; font-size: 0.95rem; }
  .empty-state p { margin: 0.24rem 0 0; color: var(--text-secondary); font-size: 0.82rem; }
  .status-note { color: var(--text-secondary); font-size: 0.86rem; margin: 0; }

  .alert {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.65rem 0.75rem;
    font-size: 0.85rem;
  }
  .alert-error {
    color: #fca5a5;
    border-color: color-mix(in srgb, #ef4444 40%, var(--border-color));
    background: color-mix(in srgb, #ef4444 8%, transparent);
  }

  /* queue-index reused in summary */
  .queue-index {
    width: 22px;
    height: 22px;
    border-radius: 7px;
    border: 1px solid var(--border-color);
    align-items: center;
    justify-content: center;
    font-size: 0.72rem;
    color: var(--text-secondary);
    font-weight: 700;
    background: var(--bg-surface);
  }

  /* ── modal styles (kept from original) ── */
  .checkout-actions { display: flex; justify-content: flex-end; gap: 0.6rem; }
  .location-form { display: grid; gap: 0.85rem; }
  .form-field { display: grid; gap: 0.35rem; }
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
  .textarea { min-height: 84px; resize: vertical; }
  .location-grid-2 { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.65rem; }

  /* ── map picker ── */
  .map-picker {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 0.7rem;
    background: var(--bg-surface);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .map-picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .map-picker-actions { display: flex; gap: 0.35rem; }
  .form-field-label {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .map-picker-canvas {
    width: 100%;
    height: 260px;
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-elevated, #f3f4f6);
    border: 1px solid var(--border-color);
  }
  .map-picker-error {
    color: var(--color-danger, #c33);
    font-size: 0.82rem;
    margin: 0;
  }
  .map-picker-coords {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    font-size: 0.82rem;
    color: var(--text-secondary);
    font-family: var(--font-mono, ui-monospace, "JetBrains Mono", monospace);
  }
  .map-picker-status {
    color: var(--text-tertiary, #6b7280);
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }

  /* ── responsive ── */
  @media (max-width: 860px) {
    .order-layout { grid-template-columns: 1fr; }
    .stepper-sidebar {
      position: static;
      flex-direction: row;
      gap: 0;
      padding: 0.4rem;
      overflow-x: auto;
    }
    .stepper-step { flex-direction: column; align-items: center; text-align: center; gap: 0.2rem; flex: 1; min-width: 0; }
    .stepper-connector { width: 18px; height: 2px; margin-left: 0; margin-top: 0.6rem; flex-shrink: 0; }
    .step-label { text-align: center; }
    .step-subtitle { display: none; }

    .cycle-toolbar { align-items: stretch; }
    .stage-nav { flex-direction: column; align-items: stretch; }
    .addr-grid { grid-template-columns: 1fr; }
    .pkg-grid { grid-template-columns: 1fr; }
    .location-grid-2 { grid-template-columns: 1fr; }
  }
</style>
