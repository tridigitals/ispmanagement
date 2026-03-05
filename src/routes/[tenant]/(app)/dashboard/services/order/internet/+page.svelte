<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import { api, type CustomerLocation, type IspPackage } from '$lib/api/client';
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

  let coverageError = $state('');
  let coverageChecking = $state(false);
  let coverageFiltering = $state(false);
  let coverageZoneId = $state<string | null>(null);
  let coverageZoneName = $state<string | null>(null);
  let coverageHasCoordinates = $state(false);
  let coverageOffersByPackage = $state<
    Record<string, { price_monthly: number | null; price_yearly: number | null }>
  >({});
  let coverageVersion = 0;

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

  type TxValues = Record<string, string | number | boolean | Date | null | undefined>;

  function tx(key: string, fallback: string, values?: TxValues) {
    return get(t)(key, values ? { values } : undefined) || fallback;
  }

  onMount(() => {
    void loadData();
  });

  $effect(() => {
    draftLocationId;
    basePackages;
    if (!loading) {
      void refreshCoverage();
    }
  });

  const selectedLocation = $derived.by(
    () => locations.find((location) => location.id === draftLocationId) || null,
  );

  const packages = $derived.by(() => {
    if (!coverageFiltering) return basePackages;
    if (!coverageZoneId) return [];
    return basePackages
      .filter((pkg) => !!coverageOffersByPackage[pkg.id])
      .map((pkg) => {
        const offer = coverageOffersByPackage[pkg.id];
        if (!offer) return pkg;
        return {
          ...pkg,
          price_monthly: offer.price_monthly ?? pkg.price_monthly,
          price_yearly: offer.price_yearly ?? pkg.price_yearly,
        };
      });
  });

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

  async function refreshCoverage() {
    const location = selectedLocation;
    const myVersion = ++coverageVersion;

    coverageError = '';
    coverageZoneId = null;
    coverageZoneName = null;
    coverageOffersByPackage = {};
    coverageHasCoordinates = false;
    coverageFiltering = false;

    if (!location) return;

    if (location.latitude == null || location.longitude == null) {
      return;
    }

    coverageHasCoordinates = true;
    coverageChecking = true;
    try {
      const result = await api.networkMapping.zones.checkCoverage({
        lat: Number(location.latitude),
        lng: Number(location.longitude),
      });
      if (myVersion !== coverageVersion) return;

      coverageZoneId = result?.zone?.id || null;
      coverageZoneName = result?.zone?.name || null;
      coverageFiltering = true;

      const map: Record<string, { price_monthly: number | null; price_yearly: number | null }> = {};
      for (const offer of result?.offers || []) {
        if (!offer?.package_id) continue;
        map[offer.package_id] = {
          price_monthly: offer.price_monthly ?? null,
          price_yearly: offer.price_yearly ?? null,
        };
      }
      coverageOffersByPackage = map;
    } catch (e: any) {
      if (myVersion !== coverageVersion) return;
      const message = String(e?.message || e || '');
      if (message.toLowerCase().includes('permission denied')) {
        coverageError = '';
      } else {
        coverageError = message || tx('dashboard.internet_order.toasts.coverage_failed', 'Gagal memeriksa cakupan');
      }
      coverageFiltering = false;
      coverageZoneId = null;
      coverageZoneName = null;
      coverageOffersByPackage = {};
    } finally {
      if (myVersion === coverageVersion) coverageChecking = false;
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
  <section class="hero card">
    <div>
      <h1>{$t('dashboard.internet_order.hero.title') || 'Pesan Layanan Internet'}</h1>
      <p>
        {$t('dashboard.internet_order.hero.subtitle') ||
          'Alur: pilih alamat, pilih paket, tinjau pesanan, lalu kirim permintaan instalasi.'}
      </p>
    </div>
    <div class="hero-actions">
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/services/order')}>
        <Icon name="arrow-left" size={15} />
        {$t('dashboard.internet_order.actions.service_types') || 'Jenis Layanan'}
      </button>
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/invoices')}>
        <Icon name="file-text" size={15} />
        {$t('dashboard.internet_order.actions.billing_invoices') || 'Tagihan & Invoice'}
      </button>
      <button class="btn btn-secondary" type="button" onclick={loadData} disabled={loading}>
        <Icon name="refresh-cw" size={15} />
        {$t('dashboard.internet_order.actions.refresh') || 'Muat Ulang'}
      </button>
    </div>
  </section>

  <section class="steps card">
    <button class={`step-pill ${step === 1 ? 'active' : ''}`} type="button" onclick={() => (step = 1)}>
      1. {$t('dashboard.internet_order.steps.address') || 'Alamat'}
    </button>
    <button class={`step-pill ${step === 2 ? 'active' : ''}`} type="button" onclick={() => step === 1 ? moveToPackageStep() : (step = 2)}>
      2. {$t('dashboard.internet_order.steps.package') || 'Paket'}
    </button>
    <button class={`step-pill ${step === 3 ? 'active' : ''}`} type="button" onclick={() => (step = 3)}>
      3. {$t('dashboard.internet_order.steps.review') || 'Tinjau'}
    </button>
  </section>

  {#if loadError}
    <section class="alert alert-error">{loadError}</section>
  {/if}
  {#if coverageError}
    <section class="alert alert-error">{coverageError}</section>
  {/if}

  {#if step === 1}
    <section class="card stage-card">
      <div class="stage-head">
        <div>
          <h3>{$t('dashboard.internet_order.stage.address.title') || 'Pilih Alamat'}</h3>
          <p>
            {$t('dashboard.internet_order.stage.address.subtitle') ||
              'Pilih alamat yang sudah ada atau tambahkan alamat baru terlebih dahulu.'}
          </p>
        </div>
        <button class="btn btn-secondary" type="button" onclick={openAddLocationModal}>
          <Icon name="map-pin" size={15} />
          {$t('dashboard.internet_order.actions.add_location') || 'Tambah Lokasi'}
        </button>
      </div>

      {#if !loading && locations.length === 0}
        <div class="empty-state">
          <Icon name="map-pin" size={18} />
          <div>
            <h4>{$t('dashboard.internet_order.empty.no_locations_title') || 'Belum ada lokasi'}</h4>
            <p>
              {$t('dashboard.internet_order.empty.no_locations_subtitle') ||
                'Tambahkan minimal satu lokasi untuk melanjutkan pemesanan internet.'}
            </p>
          </div>
        </div>
      {:else}
        <div class="field">
          <label for="draft-location">{$t('dashboard.internet_order.labels.location') || 'Lokasi'}</label>
          <select id="draft-location" bind:value={draftLocationId} disabled={loading}>
            {#each locations as location (location.id)}
              <option value={location.id}>{location.label}</option>
            {/each}
          </select>
        </div>

        <div class="coverage-box">
          <small>{$t('dashboard.internet_order.labels.coverage_zone') || 'Zona Cakupan'}</small>
          {#if coverageChecking}
            <strong>{$t('dashboard.internet_order.coverage.checking') || 'Memeriksa cakupan...'}</strong>
          {:else if !selectedLocation}
            <strong>{$t('dashboard.internet_order.coverage.select_location') || 'Pilih lokasi terlebih dahulu'}</strong>
          {:else if !coverageHasCoordinates}
            <strong>
              {$t('dashboard.internet_order.coverage.missing_coordinates') ||
                'Lokasi ini belum memiliki koordinat. Filter cakupan dinonaktifkan.'}
            </strong>
          {:else if coverageZoneId}
            <strong>{coverageZoneName || coverageZoneId}</strong>
          {:else}
            <strong>
              {$t('dashboard.internet_order.coverage.not_covered') || 'Lokasi ini di luar zona cakupan aktif.'}
            </strong>
          {/if}
        </div>

        <div class="stage-actions">
          <button class="btn btn-primary" type="button" onclick={moveToPackageStep} disabled={!draftLocationId}>
            <Icon name="arrow-right" size={15} />
            {$t('dashboard.internet_order.actions.next_choose_package') || 'Lanjut: Pilih Paket'}
          </button>
        </div>
      {/if}
    </section>
  {/if}

  {#if step === 2}
    <section class="card stage-card">
      <div class="stage-head">
        <div>
          <h3>{$t('dashboard.internet_order.stage.package.title') || 'Pilih Paket'}</h3>
          <p>
            {$t('dashboard.internet_order.labels.location') || 'Lokasi'}: <strong>{selectedLocation?.label || '-'}</strong>
          </p>
        </div>
        <button class="btn btn-secondary" type="button" onclick={moveBackToAddressStep}>
          <Icon name="arrow-left" size={15} />
          {$t('dashboard.internet_order.actions.back_to_address') || 'Kembali ke Alamat'}
        </button>
      </div>

      <div class="package-toolbar">
        <div class="field">
          <div class="field-label">{$t('dashboard.internet_order.labels.billing_cycle') || 'Siklus Tagihan'}</div>
          <div class="cycle-pills">
            <button
              class={`cycle-pill ${draftBillingCycle === 'monthly' ? 'active' : ''}`}
              type="button"
              onclick={() => (draftBillingCycle = 'monthly')}
            >
              {$t('dashboard.internet_order.cycles.monthly') || 'Bulanan'}
            </button>
            <button
              class={`cycle-pill ${draftBillingCycle === 'yearly' ? 'active' : ''}`}
              type="button"
              onclick={() => (draftBillingCycle = 'yearly')}
              disabled={!draftPackage || !hasYearlyPrice(draftPackage)}
            >
              {$t('dashboard.internet_order.cycles.yearly') || 'Tahunan'}
            </button>
          </div>
        </div>
        <div class="mini-location">
          <small>{$t('dashboard.internet_order.labels.selected_address') || 'Alamat Terpilih'}</small>
          <strong>{selectedLocation?.label || '-'}</strong>
        </div>
      </div>

      {#if loading}
        <p class="status-note">{$t('dashboard.internet_order.status.loading_packages') || 'Memuat paket...'}</p>
      {:else if packages.length === 0}
        <div class="empty-state">
          <Icon name="package" size={18} />
          <div>
            <h4>{$t('dashboard.internet_order.empty.no_packages_title') || 'Tidak ada paket tersedia untuk lokasi ini'}</h4>
            <p>{$t('dashboard.internet_order.empty.no_packages_subtitle') || 'Coba lokasi lain atau hubungi admin.'}</p>
          </div>
        </div>
      {:else}
        <div class="package-stage-grid">
          <div class="package-grid">
            {#each packages as pkg (pkg.id)}
              <article class={`package-card ${draftPackageId === pkg.id ? 'active' : ''}`}>
                <div class="package-top">
                  <h4>{pkg.name}</h4>
                  {#if draftPackageId === pkg.id}
                    <span class="badge">{$t('dashboard.internet_order.badges.selected') || 'Dipilih'}</span>
                  {/if}
                </div>
                <div class="package-price">
                  <strong>{formatCurrency(Number(pkg.price_monthly || 0))}</strong>
                  <span>{$t('dashboard.internet_order.labels.monthly_plan') || 'Paket bulanan'}</span>
                </div>
                <div class="package-extra">
                  {#if hasYearlyPrice(pkg)}
                    <span>
                      {formatCurrency(Number(pkg.price_yearly || 0))} {$t('dashboard.internet_order.labels.yearly_available') || 'opsi tahunan tersedia'}
                    </span>
                  {:else}
                    <span>{$t('dashboard.internet_order.labels.yearly_not_available') || 'Paket tahunan belum tersedia'}</span>
                  {/if}
                </div>
                <button class={`btn select-btn ${draftPackageId === pkg.id ? 'btn-primary' : 'btn-secondary'}`} type="button" onclick={() => (draftPackageId = pkg.id)}>
                  <Icon name={draftPackageId === pkg.id ? 'check-circle' : 'circle'} size={14} />
                  {draftPackageId === pkg.id
                    ? ($t('dashboard.internet_order.actions.selected') || 'Terpilih')
                    : ($t('dashboard.internet_order.actions.select_package') || 'Pilih Paket')}
                </button>
              </article>
            {/each}
          </div>

          <aside class="package-sidecard">
            <div class="sidecard-head">
              <h4>{$t('dashboard.internet_order.sidecard.title') || 'Ringkasan Pilihan'}</h4>
              <span class="sidecard-badge">{$t('dashboard.internet_order.sidecard.step_badge') || 'Langkah 2 / 3'}</span>
            </div>
            <dl class="sidecard-list">
              <div>
                <dt>{$t('dashboard.internet_order.sidecard.address') || 'Alamat'}</dt>
                <dd>{selectedLocation?.label || '-'}</dd>
              </div>
              <div>
                <dt>{$t('dashboard.internet_order.sidecard.package') || 'Paket'}</dt>
                <dd>{draftPackage?.name || '-'}</dd>
              </div>
              <div>
                <dt>{$t('dashboard.internet_order.sidecard.billing') || 'Tagihan'}</dt>
                <dd>{billingCycleLabel(draftBillingCycle)}</dd>
              </div>
            </dl>
            <div class="sidecard-total">
              <small>{$t('dashboard.internet_order.labels.current_item_total') || 'Total Item Saat Ini'}</small>
              <strong>{formatCurrency(draftAmount)}</strong>
            </div>
            <button class="btn btn-primary sidecard-cta" type="button" onclick={orderNowFromPackage}>
              <Icon name="shopping-cart" size={16} />
              {$t('dashboard.internet_order.actions.order_now') || 'Pesan Sekarang'}
            </button>
            <p class="sidecard-hint">
              {$t('dashboard.internet_order.sidecard.hint') ||
                'Anda bisa menambah layanan lain nanti pada langkah Tinjau.'}
            </p>
          </aside>
        </div>
      {/if}
    </section>
  {/if}

  {#if step === 3}
    <section class="card stage-card">
      <div class="stage-head">
        <div>
          <h3>{$t('dashboard.internet_order.stage.review.title') || 'Tinjau & Kirim'}</h3>
          <p>
            {$t('dashboard.internet_order.stage.review.subtitle') ||
              'Semua item berikut akan dikirim sebagai permintaan instalasi.'}
          </p>
        </div>
      </div>

      {#if orderItems.length === 0}
        <div class="empty-state">
          <Icon name="clipboard-list" size={18} />
          <div>
            <h4>{$t('dashboard.internet_order.empty.no_order_items_title') || 'Belum ada item pesanan'}</h4>
            <p>{$t('dashboard.internet_order.empty.no_order_items_subtitle') || 'Tambahkan minimal satu paket terlebih dahulu.'}</p>
          </div>
        </div>
        <div class="stage-actions">
          <button class="btn btn-secondary" type="button" onclick={() => (step = 2)}>
            <Icon name="arrow-left" size={15} />
            {$t('dashboard.internet_order.actions.back_to_package') || 'Kembali ke Paket'}
          </button>
        </div>
      {:else}
        <div class="queue-list">
          {#each orderItems as item, index (item.id)}
            {@const pkg = getPackageById(item.package_id)}
            <div class="queue-item">
              <div class="queue-index">{index + 1}</div>
              <div class="queue-main">
                <strong>{pkg?.name || item.package_id}</strong>
                <span>{locationLabel(item.location_id)} · {billingCycleLabel(item.billing_cycle)}</span>
              </div>
              <strong class="queue-price">{formatCurrency(getOrderItemAmount(item))}</strong>
              <button class="btn btn-secondary icon-only" type="button" onclick={() => removeOrderItem(item.id)}>
                <Icon name="trash-2" size={14} />
              </button>
            </div>
          {/each}
        </div>

        <div class="summary-row">
          <div>
            <small>{$t('dashboard.internet_order.labels.total_order_amount') || 'Total Pesanan'}</small>
            <strong>{formatCurrency(orderTotalAmount)}</strong>
          </div>
          <div class="hero-actions">
            <button class="btn btn-secondary" type="button" onclick={addMoreFromStep3}>
              <Icon name="plus" size={15} />
              {$t('dashboard.internet_order.actions.add_more') || 'Tambah Lagi'}
            </button>
            <button
              class="btn btn-primary"
              type="button"
              onclick={submitBulkOrder}
              disabled={submitLoading || orderItems.length === 0}
            >
              {#if submitLoading}
                <Icon name="refresh-cw" size={16} />
                {$t('dashboard.internet_order.status.processing') || 'Memproses...'}
              {:else}
                <Icon name="check-circle" size={16} />
                {$t('dashboard.internet_order.actions.submit_installation_request') || 'Kirim Permintaan Instalasi'}
              {/if}
            </button>
          </div>
        </div>
      {/if}
    </section>
  {/if}
</div>

<Modal
  show={showAddLocationModal}
  title={$t('dashboard.internet_order.modal.add_location_title') || 'Tambah Lokasi'}
  onclose={() => {
    if (!creatingLocation) showAddLocationModal = false;
  }}
>
  <div class="location-form">
    <label class="form-field">
      <span>{$t('dashboard.internet_order.modal.fields.label') || 'Label'}</span>
      <input class="input" bind:value={newLocationLabel} placeholder={$t('dashboard.internet_order.modal.placeholders.label') || 'Rumah / Kantor'} />
    </label>
    <label class="form-field">
      <span>{$t('dashboard.internet_order.modal.fields.address') || 'Alamat'}</span>
      <input class="input" bind:value={newLocationAddress} placeholder={$t('dashboard.internet_order.modal.placeholders.address') || 'Alamat jalan'} />
    </label>
    <div class="location-grid-2">
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.city') || 'Kota'}</span>
        <input class="input" bind:value={newLocationCity} />
      </label>
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.state') || 'Provinsi'}</span>
        <input class="input" bind:value={newLocationState} />
      </label>
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.postal_code') || 'Kode Pos'}</span>
        <input class="input" bind:value={newLocationPostalCode} />
      </label>
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.country') || 'Negara'}</span>
        <input class="input" bind:value={newLocationCountry} />
      </label>
    </div>
    <label class="form-field">
      <span>{$t('dashboard.internet_order.modal.fields.notes') || 'Catatan'}</span>
      <textarea class="input textarea" bind:value={newLocationNotes} rows="3"></textarea>
    </label>
    <div class="location-grid-2">
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.latitude') || 'Latitude'}</span>
        <input class="input" bind:value={newLocationLatitude} placeholder="-6.200000" />
      </label>
      <label class="form-field">
        <span>{$t('dashboard.internet_order.modal.fields.longitude') || 'Longitude'}</span>
        <input class="input" bind:value={newLocationLongitude} placeholder="106.816666" />
      </label>
    </div>
    <div class="checkout-actions">
      <button
        class="btn btn-secondary"
        onclick={() => (showAddLocationModal = false)}
        disabled={creatingLocation}
      >
        {$t('dashboard.internet_order.actions.cancel') || 'Batal'}
      </button>
      <button class="btn btn-primary" onclick={saveMyLocation} disabled={creatingLocation || !newLocationLabel.trim()}>
        {#if creatingLocation}
          <Icon name="refresh-cw" size={16} />
          {$t('dashboard.internet_order.status.saving') || 'Menyimpan...'}
        {:else}
          <Icon name="save" size={16} />
          {$t('dashboard.internet_order.actions.save') || 'Simpan'}
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

  .card {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-surface);
    padding: 1rem;
  }

  .hero {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    flex-wrap: wrap;
  }

  .hero h1 {
    margin: 0;
    font-size: clamp(1.25rem, 2.2vw, 1.6rem);
  }

  .hero p {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
    max-width: 780px;
  }

  .hero-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .steps {
    display: inline-flex;
    gap: 0.52rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .step-pill {
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.32rem 0.78rem;
    font-size: 0.78rem;
    font-weight: 700;
    cursor: pointer;
  }

  .step-pill.active {
    border-color: color-mix(in srgb, var(--accent-primary) 56%, var(--border-color));
    background: color-mix(in srgb, var(--accent-primary) 14%, transparent);
    color: var(--accent-primary);
  }

  .stage-card {
    display: grid;
    gap: 0.8rem;
  }

  .stage-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .stage-head h3 {
    margin: 0;
    font-size: 1.03rem;
  }

  .stage-head p {
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .package-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: end;
    gap: 0.7rem;
    flex-wrap: wrap;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-secondary) 72%, transparent);
    padding: 0.72rem;
  }

  .mini-location {
    display: grid;
    gap: 0.12rem;
    min-width: 220px;
  }

  .mini-location small {
    color: var(--text-secondary);
    font-size: 0.73rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .mini-location strong {
    font-size: 0.92rem;
    line-height: 1.25;
  }

  .field label {
    color: var(--text-secondary);
    font-size: 0.84rem;
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

  .field-label {
    font-size: 0.74rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--text-secondary);
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

  .coverage-box {
    border: 1px dashed var(--border-color);
    border-radius: 10px;
    padding: 0.65rem;
    display: grid;
    gap: 0.22rem;
  }

  .coverage-box small {
    color: var(--text-secondary);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .coverage-box strong {
    font-size: 0.84rem;
    line-height: 1.35;
    color: var(--text-primary);
  }

  .package-stage-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 0.85rem;
    align-items: start;
  }

  .package-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 0.7rem;
  }

  .package-card {
    display: grid;
    gap: 0.62rem;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 0.8rem;
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--accent-primary) 6%, transparent) 0%,
        color-mix(in srgb, var(--bg-secondary) 82%, transparent) 48%
      );
    transition: border-color 0.2s ease, transform 0.2s ease, box-shadow 0.2s ease;
  }

  .package-card.active {
    border-color: color-mix(in srgb, var(--accent-primary) 60%, var(--border-color));
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--accent-primary) 28%, transparent),
      0 12px 22px color-mix(in srgb, #000 22%, transparent);
    transform: translateY(-1px);
  }

  .package-top {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.5rem;
    align-items: center;
  }

  .package-top h4 {
    margin: 0;
    font-size: 0.96rem;
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

  .package-price {
    display: grid;
    gap: 0.12rem;
  }

  .package-price strong {
    font-size: 1.13rem;
    line-height: 1.1;
  }

  .package-price span {
    color: var(--text-secondary);
    font-size: 0.78rem;
  }

  .package-extra {
    color: var(--text-secondary);
    font-size: 0.79rem;
    line-height: 1.35;
  }

  .select-btn {
    min-height: 38px;
    font-size: 0.84rem;
  }

  .package-sidecard {
    border: 1px solid color-mix(in srgb, var(--accent-primary) 28%, var(--border-color));
    border-radius: 12px;
    padding: 0.85rem;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--accent-primary) 10%, transparent),
      color-mix(in srgb, var(--bg-surface) 86%, transparent)
    );
    display: grid;
    gap: 0.72rem;
    position: sticky;
    top: 0.8rem;
  }

  .sidecard-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.45rem;
  }

  .sidecard-head h4 {
    margin: 0;
    font-size: 0.95rem;
  }

  .sidecard-badge {
    border: 1px solid color-mix(in srgb, var(--accent-primary) 45%, var(--border-color));
    color: var(--accent-primary);
    border-radius: 999px;
    padding: 0.14rem 0.48rem;
    font-size: 0.69rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .sidecard-list {
    margin: 0;
    display: grid;
    gap: 0.5rem;
  }

  .sidecard-list > div {
    border: 1px solid var(--border-color);
    border-radius: 9px;
    padding: 0.45rem 0.58rem;
    display: grid;
    gap: 0.1rem;
    background: color-mix(in srgb, var(--bg-surface) 88%, transparent);
  }

  .sidecard-list dt {
    margin: 0;
    font-size: 0.71rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.035em;
    font-weight: 700;
  }

  .sidecard-list dd {
    margin: 0;
    font-size: 0.88rem;
    font-weight: 600;
  }

  .sidecard-total {
    border-top: 1px dashed var(--border-color);
    padding-top: 0.64rem;
    display: grid;
    gap: 0.18rem;
  }

  .sidecard-total small {
    color: var(--text-secondary);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .sidecard-total strong {
    font-size: 1.25rem;
    line-height: 1.1;
  }

  .sidecard-cta {
    min-height: 42px;
    font-size: 0.9rem;
  }

  .sidecard-hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.77rem;
    line-height: 1.4;
  }

  .summary-row {
    border-top: 1px dashed var(--border-color);
    padding-top: 0.72rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.65rem;
    flex-wrap: wrap;
  }

  .summary-row small {
    color: var(--text-secondary);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    display: block;
  }

  .summary-row strong {
    font-size: 1.2rem;
    line-height: 1.1;
  }

  .queue-list {
    display: grid;
    gap: 0.52rem;
  }

  .queue-item {
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr) auto auto;
    gap: 0.5rem;
    align-items: center;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.58rem 0.62rem;
    background: color-mix(in srgb, var(--bg-secondary) 70%, transparent);
  }

  .queue-index {
    width: 22px;
    height: 22px;
    border-radius: 7px;
    border: 1px solid var(--border-color);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.72rem;
    color: var(--text-secondary);
    font-weight: 700;
    background: var(--bg-surface);
  }

  .queue-main {
    display: grid;
    gap: 0.14rem;
  }

  .queue-main span {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .queue-price {
    font-size: 0.93rem;
  }

  .empty-state {
    border: 1px dashed var(--border-color);
    border-radius: 10px;
    padding: 0.72rem;
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
  }

  .empty-state h4 {
    margin: 0;
    font-size: 0.95rem;
  }

  .empty-state p {
    margin: 0.24rem 0 0;
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .status-note {
    color: var(--text-secondary);
    font-size: 0.86rem;
    margin: 0;
  }

  .stage-actions {
    border-top: 1px dashed var(--border-color);
    padding-top: 0.72rem;
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

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

  .icon-only {
    min-width: 38px;
    min-height: 38px;
    padding: 0;
  }

  .checkout-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
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

  @media (max-width: 860px) {
    .package-toolbar {
      align-items: stretch;
    }

    .mini-location {
      min-width: 0;
      width: 100%;
    }

    .package-stage-grid {
      grid-template-columns: 1fr;
    }

    .package-sidecard {
      position: static;
    }

    .queue-item {
      grid-template-columns: 26px 1fr auto;
      align-items: start;
    }

    .queue-price {
      grid-column: 2;
    }

    .icon-only {
      grid-column: 3;
      grid-row: 1 / span 2;
    }

    .summary-row,
    .stage-actions {
      flex-direction: column;
      align-items: stretch;
    }

    .location-grid-2 {
      grid-template-columns: 1fr;
    }
  }
</style>
