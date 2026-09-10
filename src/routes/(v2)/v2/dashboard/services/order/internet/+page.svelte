<script lang="ts">
  /*
    Pesanan Internet portal v2 — gelombang 25c.
    Versi lama: (app)/dashboard/services/order/internet/+page.svelte (1.262 baris).
    Perilaku identik: wizard 3 langkah (alamat → paket → review) + modal
    tambah lokasi dengan map picker + submit bulk order.
    Pola DS: PortalShell + Card + Button + Badge + Field.
    Modal + Select2 reuse dari ui/ (DS belum punya modal/country picker).
  */
  import { goto } from '$app/navigation';
  import { onDestroy, onMount } from 'svelte';
  import { api, type CustomerLocation, type IspPackage } from '$lib/api/client';
  import { getVisibleInternetOrderPackages } from '$lib/utils/internetOrderPackages';
  import { buildCountryOptions } from '$lib/utils/countryOptions';
  import { appSettings } from '$lib/stores/settings';
  import { toast } from '$lib/stores/toast';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';

  import { t } from 'svelte-i18n';
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

  const countryOptions = buildCountryOptions();

  // ── Map picker state ──────────────────────────────────────────────
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

  onMount(() => {
    void loadData();
  });

  async function ensureMapPickerReady() {
    if (pickerMapInstance) return;
    if (!mapPickerContainer) {
      await new Promise((r) => setTimeout(r, 50));
      if (!mapPickerContainer) {
        mapPickerError = $t('dashboard.services_portal.order.map_noload');
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
          layers: [{ id: 'osm-raster-layer', type: 'raster', source: 'osm-raster' }],
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

      if (hasMarker) {
        pickerMarkerInstance = new maplibre.Marker({ draggable: true, color: '#d33' })
          .setLngLat([initialLng, initialLat])
          .addTo(map);
        pickerMarkerInstance.on('dragend', () => {
          const ll = pickerMarkerInstance.getLngLat();
          syncFromLngLat(ll.lng, ll.lat);
        });
      }

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
      mapPickerError = $t('dashboard.services_portal.order.map_fail');
    }
  }

  function teardownMapPicker() {
    try {
      if (pickerMarkerInstance) pickerMarkerInstance.remove();
    } catch {
      /* ignore */
    }
    try {
      if (pickerMapInstance) pickerMapInstance.remove();
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
      toast.error($t('dashboard.services_portal.order.t_geo_unsupported'));
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
      toast.error($t('dashboard.services_portal.order.t_geo_denied'));
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
      toast.error($t('dashboard.services_portal.order.t_catalog_fail'));
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
    return cycle === 'yearly' ? $t('dashboard.services_portal.order.yearly') : $t('dashboard.services_portal.order.monthly');
  }

  function locationLabel(locationId: string) {
    return locations.find((l) => l.id === locationId)?.label || locationId;
  }

  function getOrderItemAmount(item: { package_id: string; billing_cycle: 'monthly' | 'yearly' }) {
    const pkg = getPackageById(item.package_id);
    if (!pkg) return 0;
    if (item.billing_cycle === 'yearly' && hasYearlyPrice(pkg)) return Number(pkg.price_yearly || 0);
    return Number(pkg.price_monthly || 0);
  }

  function checkoutEligibilityError(pkg: IspPackage, cycle: 'monthly' | 'yearly', locationId?: string): string | null {
    const targetLocationId = locationId || draftLocationId;
    if (!targetLocationId) return $t('dashboard.services_portal.order.need_addr');
    if (cycle === 'yearly' && !hasYearlyPrice(pkg)) return $t('dashboard.services_portal.order.no_yearly');
    return null;
  }

  function moveToPackageStep() {
    if (!draftLocationId) {
      toast.error($t('dashboard.services_portal.order.need_addr'));
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
      toast.error($t('dashboard.services_portal.order.t_pkg_invalid'));
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
    toast.success($t('dashboard.services_portal.order.t_item_added'));
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
        toast.error($t('dashboard.services_portal.order.t_invalid_in_cart'));
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
          ? `Permintaan instalasi berhasil dikirim (${created[0].work_order_id || created[0].id})`
          : `${created.length} permintaan instalasi berhasil dikirim`,
      );
      orderItems = [];
      await goto('/v2/dashboard/services');
    } catch (e: any) {
      toast.error(e?.message || 'Gagal mengirim permintaan instalasi');
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
      toast.error($t('dashboard.services_portal.order.t_lat_range'));
      return;
    }
    if (lngRaw && (Number.isNaN(parsedLng) || parsedLng < -180 || parsedLng > 180)) {
      toast.error($t('dashboard.services_portal.order.t_lng_range'));
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
      toast.success($t('dashboard.services_portal.order.t_loc_saved'));
      showAddLocationModal = false;
      await loadData();
      if (locations.length > 0) draftLocationId = locations[0].id;
    } catch (e: any) {
      toast.error(e?.message || 'Gagal menambahkan lokasi');
    } finally {
      creatingLocation = false;
    }
  }
</script>
<PortalShell title={ $t('dashboard.services_portal.order.title') }>
  <PageHeader
    title={ $t('dashboard.services_portal.order.aria') }
    desc={ $t('dashboard.services_portal.order.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" icon="chevronLeft" onclick={() => goto('/v2/dashboard/services/order')}>
        Jenis Layanan
      </Button>
      <Button variant="ghost" icon="receipt" onclick={() => goto('/v2/dashboard/invoices')}>Tagihan</Button>
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={loadData}>{ $t('common.refresh') }</Button>
    {/snippet}
  </PageHeader>

  {#if loadError}
    <div class="banner-bad">
      <span>{loadError}</span>
      <Button variant="ghost" size="sm" onclick={loadData}>{ $t('common.retry') }</Button>
    </div>
  {/if}

  <div class="order-layout">
    <nav class="stepper-sidebar">
      <button class="stepper-step {step === 1 ? 'active' : ''}" type="button" onclick={() => (step = 1)}>
        <span class="step-bullet">1</span>
        <div class="step-label">
          <span class="step-title">Alamat</span>
          <span class="step-subtitle">{ $t('dashboard.services_portal.order.step_addr') }</span>
        </div>
      </button>
      <div class="stepper-connector {step >= 2 ? 'filled' : ''}"></div>
      <button class="stepper-step {step === 2 ? 'active' : ''}" type="button" onclick={moveToPackageStep} disabled={!draftLocationId}>
        <span class="step-bullet">2</span>
        <div class="step-label">
          <span class="step-title">Paket</span>
          <span class="step-subtitle">{ $t('dashboard.services_portal.order.step_pkg') }</span>
        </div>
      </button>
      <div class="stepper-connector {step >= 3 ? 'filled' : ''}"></div>
      <button class="stepper-step {step === 3 ? 'active' : ''}" type="button" onclick={() => orderItems.length > 0 && (step = 3)} disabled={orderItems.length === 0}>
        <span class="step-bullet">3</span>
        <div class="step-label">
          <span class="step-title">{ $t('dashboard.services_portal.order.review_short') }</span>
          <span class="step-subtitle">{ $t('dashboard.services_portal.order.step_review') }</span>
        </div>
      </button>
    </nav>

    <div class="order-content">
      {#if step === 1}
        <Card title={ $t('dashboard.services_portal.order.pick_addr') } padded={false}>
          {#snippet aside()}
            <Button variant="secondary" icon="pin" onclick={openAddLocationModal}>{ $t('dashboard.services_portal.order.add_loc') }</Button>
          {/snippet}
          {#if !loading && locations.length === 0}
            <div class="empty-block">
              <p class="font-medium">{ $t('dashboard.services_portal.order.no_locs') }</p>
              <p class="text-sm text-ink-500">{ $t('dashboard.services_portal.order.no_locs_hint') }</p>
            </div>
          {:else}
            <div class="addr-grid">
              {#each locations as location (location.id)}
                <button
                  class="addr-option {draftLocationId === location.id ? 'selected' : ''}"
                  type="button"
                  onclick={() => (draftLocationId = location.id)}
                >
                  <div class="addr-option-head">
                    <span class="addr-option-label">{location.label}</span>
                    {#if draftLocationId === location.id}
                      <Badge tone="positive" label={ $t('dashboard.services_portal.order.picked') } />
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
              <Button icon="chevronRight" onclick={moveToPackageStep} disabled={!draftLocationId}>
                Lanjut ke Paket
              </Button>
            </div>
          {/if}
        </Card>
      {:else if step === 2}
        <Card title={ $t('dashboard.services_portal.order.pick_pkg') } padded={false}>
          {#snippet aside()}
            <span class="text-sm text-ink-500">
              Lokasi: <strong class="text-ink-800">{selectedLocation?.label || '-'}</strong>
            </span>
          {/snippet}
          <div class="cycle-toolbar">
            <div class="cycle-pills">
              <button class="cycle-pill {draftBillingCycle === 'monthly' ? 'active' : ''}" type="button" onclick={() => (draftBillingCycle = 'monthly')}>
                { $t('dashboard.services_portal.order.monthly') }
              </button>
              <button class="cycle-pill {draftBillingCycle === 'yearly' ? 'active' : ''}" type="button" onclick={() => (draftBillingCycle = 'yearly')} disabled={!draftPackage || !hasYearlyPrice(draftPackage)}>
                { $t('dashboard.services_portal.order.yearly') }
              </button>
            </div>
          </div>

          {#if loading}
            <p class="status-note">{ $t('dashboard.services_portal.order.load_pkgs') }</p>
          {:else if packages.length === 0}
            <div class="empty-block">
              <p class="font-medium">{ $t('dashboard.services_portal.order.no_pkgs') }</p>
              <p class="text-sm text-ink-500">{ $t('dashboard.services_portal.order.no_pkgs_hint') }</p>
            </div>
          {:else}
            <div class="pkg-grid">
              {#each packages as pkg (pkg.id)}
                <button class="pkg-option {draftPackageId === pkg.id ? 'selected' : ''}" type="button" onclick={() => (draftPackageId = pkg.id)}>
                  <div class="pkg-option-head">
                    <h4>{pkg.name}</h4>
                    {#if draftPackageId === pkg.id}
                      <Badge tone="positive" label={ $t('dashboard.services_portal.order.picked') } />
                    {/if}
                  </div>
                  {#if pkg.description}
                    <p class="pkg-option-desc">{pkg.description}</p>
                  {/if}
                  <div class="pkg-option-price">
                    <strong>{formatCurrency(Number(pkg.price_monthly || 0))}</strong>
                    <span>/bulan</span>
                  </div>
                  {#if hasYearlyPrice(pkg)}
                    <div class="pkg-option-yearly">{formatCurrency(Number(pkg.price_yearly || 0))} tersedia per tahun</div>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}

          <div class="stage-nav">
            <Button variant="ghost" icon="chevronLeft" onclick={moveBackToAddressStep}>{ $t('dashboard.services_portal.order.back_addr') }</Button>
            <Button icon="chevronRight" onclick={orderNowFromPackage} disabled={!draftPackageId}>{ $t('dashboard.services_portal.order.order_now') }</Button>
          </div>
        </Card>
      {:else}
        <Card title={ $t('dashboard.services_portal.order.review') } padded={false}>
          {#if orderItems.length === 0}
            <div class="empty-block">
              <p class="font-medium">{ $t('dashboard.services_portal.order.cart_empty') }</p>
              <p class="text-sm text-ink-500">{ $t('dashboard.services_portal.order.cart_empty_hint') }</p>
            </div>
          {:else}
            <div class="summary-card">
              {#each orderItems as item, index (item.id)}
                {@const pkg = getPackageById(item.package_id)}
                <div class="summary-row">
                  <span class="summary-label">
                    <span class="queue-index">{index + 1}</span>
                    {pkg?.name || item.package_id}
                  </span>
                  <span class="summary-value">{formatCurrency(getOrderItemAmount(item))}</span>
                </div>
                <div class="summary-row sub">
                  <span>{locationLabel(item.location_id)} · {billingCycleLabel(item.billing_cycle)}</span>
                  <Button variant="ghost" size="sm" icon="close" label="Hapus" onclick={() => removeOrderItem(item.id)} />
                </div>
              {/each}
              <div class="summary-row total">
                <span class="summary-label">{ $t('dashboard.services_portal.order.cart_total') }</span>
                <span class="summary-value">{formatCurrency(orderTotalAmount)}</span>
              </div>
            </div>
          {/if}
          <div class="stage-nav">
            <Button variant="secondary" icon="plus" onclick={addMoreFromStep3}>{ $t('dashboard.services_portal.order.add_item') }</Button>
            <Button loading={submitLoading} icon="check" onclick={submitBulkOrder} disabled={orderItems.length === 0}>
              {submitLoading ? 'Memproses…' : $t('dashboard.services_portal.order.send_req')}
            </Button>
          </div>
        </Card>
      {/if}
    </div>
  </div>
</PortalShell>
<Modal
  show={showAddLocationModal}
  title={ $t('dashboard.services_portal.order.add_loc') }
  width="640px"
  onclose={() => {
    if (!creatingLocation) showAddLocationModal = false;
  }}
>
  <div class="location-form">
    <label class="form-field">
      <span>Label</span>
      <input class="input" bind:value={newLocationLabel} placeholder={ $t('dashboard.services_portal.order.label_ph') } />
    </label>
    <label class="form-field">
      <span>Alamat</span>
      <input class="input" bind:value={newLocationAddress} placeholder={ $t('dashboard.services_portal.order.addr_ph') } />
    </label>
    <div class="location-grid-2">
      <label class="form-field">
        <span>Kota</span>
        <input class="input" bind:value={newLocationCity} />
      </label>
      <label class="form-field">
        <span>Provinsi</span>
        <input class="input" bind:value={newLocationState} />
      </label>
      <label class="form-field">
        <span>{ $t('dashboard.services_portal.order.postal') }</span>
        <input class="input" bind:value={newLocationPostalCode} />
      </label>
      <label class="form-field">
        <span>Negara</span>
        <Select2
          bind:value={newLocationCountry}
          options={countryOptions}
          placeholder={ $t('dashboard.services_portal.order.pick_country') }
          searchPlaceholder="Cari negara…"
          noResultsText="Negara tidak ditemukan"
          maxItems={50}
        />
      </label>
    </div>
    <label class="form-field">
      <span>Catatan</span>
      <textarea class="input textarea" bind:value={newLocationNotes} rows="3"></textarea>
    </label>

    <div class="map-picker">
      <div class="map-picker-header">
        <span class="form-field-label">{ $t('dashboard.services_portal.order.map_hint') }</span>
        <div class="map-picker-actions">
          <button
            type="button"
            class="btn-secondary-sm"
            onclick={detectMyLocation}
            disabled={mapPickerDetecting}
          >
            <Icon name={mapPickerDetecting ? 'refresh-cw' : 'crosshair'} size={14} />
            Gunakan lokasi saya
          </button>
        </div>
      </div>
      <div
        class="map-picker-canvas"
        bind:this={mapPickerContainer}
        role="application"
        aria-label={ $t('dashboard.services_portal.order.map_aria') }
      ></div>
      {#if mapPickerError}
        <p class="map-picker-error">{mapPickerError}</p>
      {/if}
      <div class="map-picker-coords">
        <span><strong>Lat:</strong> {newLocationLatitude || '—'}</span>
        <span><strong>Lng:</strong> {newLocationLongitude || '—'}</span>
        {#if !mapPickerMounted && !mapPickerError}
          <span class="map-picker-status">
            <Icon name="refresh-cw" size={12} />
            Memuat peta…
          </span>
        {/if}
      </div>
    </div>
    <div class="checkout-actions">
      <Button variant="secondary" onclick={() => (showAddLocationModal = false)} disabled={creatingLocation}>
        { $t('dashboard.services_portal.order.cancel_btn') }
      </Button>
      <Button loading={creatingLocation} icon="check" disabled={!newLocationLabel.trim()}>
        {creatingLocation ? $t('dashboard.services_portal.order.saving_btn') : $t('dashboard.services_portal.order.save_btn')}
      </Button>
    </div>
  </div>
</Modal>

<style>
  .banner-bad {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    border-radius: 0.75rem;
    border: 1px solid #fecaca;
    background: #fef2f2;
    color: #991b1b;
    padding: 0.7rem 1rem;
    font-size: 0.875rem;
  }
  .order-layout {
    display: grid;
    grid-template-columns: 230px minmax(0, 1fr);
    gap: 1.2rem;
    align-items: start;
  }
  @media (max-width: 900px) {
    .order-layout { grid-template-columns: 1fr; }
  }
  .stepper-sidebar {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--ink-200, #e7e5e4);
    border-radius: 14px;
    background: #fff;
    padding: 0.8rem;
    position: sticky;
    top: 0.8rem;
  }
  @media (max-width: 900px) {
    .stepper-sidebar { position: static; }
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
  }
  .stepper-step:disabled { opacity: 0.45; cursor: not-allowed; }
  .stepper-step.active { background: var(--ink-100, #f5f5f4); }
  .step-bullet {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid var(--ink-200, #e7e5e4);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.78rem;
    font-weight: 700;
    flex-shrink: 0;
    color: var(--ink-500, #78716c);
    background: #fff;
  }
  .stepper-step.active .step-bullet {
    border-color: var(--brand, #2563eb);
    color: var(--brand, #2563eb);
  }
  .step-label { display: flex; flex-direction: column; }
  .step-title { font-size: 0.85rem; font-weight: 650; color: var(--ink-800, #292524); }
  .step-subtitle { font-size: 0.72rem; color: var(--ink-400, #a8a29e); }
  .stepper-connector {
    width: 2px;
    height: 14px;
    background: var(--ink-200, #e7e5e4);
    margin-left: 13px;
  }
  .stepper-connector.filled { background: var(--brand, #2563eb); }
  .order-content { min-width: 0; }
  .empty-block { padding: 1.5rem 1rem; display: grid; gap: 0.35rem; justify-items: center; text-align: center; }
  .stage-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.85rem 1rem;
    border-top: 1px solid var(--ink-100, #f5f5f4);
    flex-wrap: wrap;
  }
  .addr-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 0.75rem;
    padding: 1rem;
  }
  .addr-option {
    text-align: left;
    border: 1px solid var(--ink-200, #e7e5e4);
    border-radius: 12px;
    background: #fff;
    padding: 0.85rem;
    cursor: pointer;
    display: grid;
    gap: 0.3rem;
  }
  .addr-option.selected {
    border-color: var(--brand, #2563eb);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--brand, #2563eb) 22%, transparent);
  }
  .addr-option-head { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  .addr-option-label { font-weight: 650; font-size: 0.9rem; }
  .addr-option-detail { margin: 0; font-size: 0.8rem; color: var(--ink-600, #57534e); }
  .addr-option-meta { margin: 0; font-size: 0.72rem; color: var(--ink-400, #a8a29e); }
  .cycle-toolbar { padding: 0.85rem 1rem 0; }
  .cycle-pills { display: inline-flex; border: 1px solid var(--ink-200, #e7e5e4); border-radius: 9999px; overflow: hidden; }
  .cycle-pill {
    border: none;
    background: #fff;
    padding: 0.4rem 1rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--ink-500, #78716c);
    cursor: pointer;
  }
  .cycle-pill:disabled { opacity: 0.4; cursor: not-allowed; }
  .cycle-pill.active { background: var(--brand, #2563eb); color: #fff; }
  .status-note { padding: 1rem; color: var(--ink-500, #78716c); font-size: 0.85rem; }
  .pkg-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
    padding: 1rem;
  }
  .pkg-option {
    text-align: left;
    border: 1px solid var(--ink-200, #e7e5e4);
    border-radius: 12px;
    background: #fff;
    padding: 0.9rem;
    cursor: pointer;
    display: grid;
    gap: 0.4rem;
  }
  .pkg-option.selected {
    border-color: var(--brand, #2563eb);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--brand, #2563eb) 22%, transparent);
  }
  .pkg-option-head { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  .pkg-option-head h4 { margin: 0; font-size: 0.95rem; font-weight: 650; }
  .pkg-option-desc { margin: 0; font-size: 0.78rem; color: var(--ink-500, #78716c); }
  .pkg-option-price { font-size: 0.85rem; color: var(--ink-500, #78716c); }
  .pkg-option-price strong { font-size: 1.05rem; color: var(--ink-900, #1c1917); }
  .pkg-option-yearly { font-size: 0.72rem; color: var(--ink-400, #a8a29e); }
  .summary-card { display: grid; gap: 0.5rem; padding: 1rem; }
  .summary-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }
  .summary-row.sub { font-size: 0.82rem; color: var(--ink-500, #78716c); }
  .summary-row.total {
    border-top: 1px dashed var(--ink-200, #e7e5e4);
    margin-top: 0.4rem;
    padding-top: 0.6rem;
  }
  .summary-row.total .summary-label { font-weight: 700; }
  .summary-row.total .summary-value { font-size: 1.15rem; font-weight: 700; color: var(--brand, #2563eb); }
  .summary-label { display: inline-flex; align-items: center; gap: 0.45rem; font-size: 0.88rem; }
  .summary-value { font-weight: 650; }
  .queue-index {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--ink-100, #f5f5f4);
    font-size: 0.68rem;
    font-weight: 700;
  }
  .location-form { display: grid; gap: 0.85rem; }
  .form-field { display: grid; gap: 0.3rem; font-size: 0.85rem; }
  .form-field > span { font-weight: 600; color: var(--ink-700, #44403c); }
  .input {
    border: 1px solid var(--ink-200, #e7e5e4);
    border-radius: 8px;
    padding: 0.5rem 0.65rem;
    font-size: 0.85rem;
    width: 100%;
    background: #fff;
  }
  .textarea { resize: vertical; }
  .location-grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 0.7rem; }
  .map-picker { display: grid; gap: 0.5rem; }
  .map-picker-header { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; flex-wrap: wrap; }
  .form-field-label { font-weight: 600; font-size: 0.85rem; color: var(--ink-700, #44403c); }
  .btn-secondary-sm {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fff;
    border-radius: 8px;
    padding: 0.35rem 0.65rem;
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--ink-700, #44403c);
    cursor: pointer;
  }
  .btn-secondary-sm:disabled { opacity: 0.5; cursor: not-allowed; }
  .map-picker-canvas {
    height: 300px;
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: var(--ink-100, #f5f5f4);
  }
  .map-picker-error { margin: 0; font-size: 0.8rem; color: #dc2626; }
  .map-picker-coords { display: flex; gap: 1rem; font-size: 0.78rem; color: var(--ink-500, #78716c); flex-wrap: wrap; }
  .map-picker-status { display: inline-flex; align-items: center; gap: 0.3rem; }
  .checkout-actions { display: flex; justify-content: flex-end; gap: 0.6rem; }
</style>
