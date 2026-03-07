<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { api, type CustomerLocation } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';

  let loading = $state(true);
  let locations = $state<CustomerLocation[]>([]);
  let error = $state('');

  let showLocationModal = $state(false);
  let editingLocation: CustomerLocation | null = $state(null);
  let savingLocation = $state(false);
  let showDeleteDialog = $state(false);
  let deletingLocation = $state(false);
  let deleteLocationId = $state<string | null>(null);

  let fLabel = $state('');
  let fLine1 = $state('');
  let fLine2 = $state('');
  let fCity = $state('');
  let fState = $state('');
  let fPostal = $state('');
  let fCountry = $state('ID');
  let fNotes = $state('');
  let fLatitude = $state('');
  let fLongitude = $state('');

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

  const hasLinkedCustomer = $derived($can('read_own', 'customers'));
  const totalLocations = $derived(locations.length);
  const mappedLocations = $derived(
    locations.filter((loc) => loc.latitude != null && loc.longitude != null).length,
  );
  const notedLocations = $derived(locations.filter((loc) => (loc.notes || '').trim().length > 0).length);

  onMount(async () => {
    await load();
  });

  onDestroy(() => {
    pickerMarker?.remove();
    pickerMap?.remove();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      locations = hasLinkedCustomer ? await api.customers.portal.myLocations() : [];
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to load locations');
      toast.error(get(t)('dashboard.locations.toasts.load_failed') || 'Failed to load locations');
    } finally {
      loading = false;
    }
  }

  function formatAddress(loc: CustomerLocation) {
    return [
      loc.address_line1,
      loc.address_line2,
      [loc.city, loc.state, loc.postal_code].filter(Boolean).join(', '),
      loc.country,
    ]
      .filter((part) => Boolean(part && String(part).trim()))
      .join(' • ');
  }

  function resetForm() {
    editingLocation = null;
    fLabel = '';
    fLine1 = '';
    fLine2 = '';
    fCity = '';
    fState = '';
    fPostal = '';
    fCountry = 'ID';
    fNotes = '';
    fLatitude = '';
    fLongitude = '';
  }

  function openCreateLocation() {
    resetForm();
    showLocationModal = true;
  }

  function openEditLocation(loc: CustomerLocation) {
    editingLocation = loc;
    fLabel = loc.label || '';
    fLine1 = loc.address_line1 || '';
    fLine2 = loc.address_line2 || '';
    fCity = loc.city || '';
    fState = loc.state || '';
    fPostal = loc.postal_code || '';
    fCountry = loc.country || 'ID';
    fNotes = loc.notes || '';
    fLatitude = loc.latitude != null ? String(loc.latitude) : '';
    fLongitude = loc.longitude != null ? String(loc.longitude) : '';
    showLocationModal = true;
  }

  function parseCoordOrNull(v: string) {
    const raw = v.trim();
    if (!raw) return null;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : NaN;
  }

  function validateLocationForm() {
    const label = fLabel.trim();
    if (!label) {
      toast.error('Label lokasi wajib diisi');
      return null;
    }

    const latitude = parseCoordOrNull(fLatitude);
    const longitude = parseCoordOrNull(fLongitude);
    if (latitude == null || longitude == null) {
      toast.error('Lokasi wajib dipilih di map');
      return null;
    }
    if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) {
      toast.error('Koordinat lokasi tidak valid');
      return null;
    }
    if (latitude < -90 || latitude > 90) {
      toast.error('Latitude harus di antara -90 hingga 90');
      return null;
    }
    if (longitude < -180 || longitude > 180) {
      toast.error('Longitude harus di antara -180 hingga 180');
      return null;
    }

    return {
      label,
      address_line1: fLine1.trim() || null,
      address_line2: fLine2.trim() || null,
      city: fCity.trim() || null,
      state: fState.trim() || null,
      postal_code: fPostal.trim() || null,
      country: fCountry.trim().toUpperCase() || null,
      notes: fNotes.trim() || null,
      latitude,
      longitude,
    };
  }

  async function saveLocation() {
    const payload = validateLocationForm();
    if (!payload) return;

    savingLocation = true;
    try {
      if (editingLocation) {
        await api.customers.portal.updateMyLocation(editingLocation.id, payload);
      } else {
        await api.customers.portal.createMyLocation(payload);
      }
      showLocationModal = false;
      resetForm();
      await load();
      toast.success($t('common.saved') || 'Saved');
    } catch (e: any) {
      toast.error(String(e?.message || e || 'Failed to save location'));
    } finally {
      savingLocation = false;
    }
  }

  function askDeleteLocation(locationId: string) {
    deleteLocationId = locationId;
    showDeleteDialog = true;
  }

  async function doDeleteLocation() {
    if (!deleteLocationId) return;
    deletingLocation = true;
    try {
      await api.customers.portal.deleteMyLocation(deleteLocationId);
      showDeleteDialog = false;
      deleteLocationId = null;
      await load();
      toast.success($t('common.deleted') || 'Deleted');
    } catch (e: any) {
      toast.error(String(e?.message || e || 'Failed to delete location'));
    } finally {
      deletingLocation = false;
    }
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
    setVisibility('location-picker-standard', !showSatellite);
    setVisibility('location-picker-satellite', showSatellite);
    const targetMaxZoom = showSatellite ? pickerSatelliteMaxZoom : pickerStandardMaxZoom;
    pickerMap.setMaxZoom(targetMaxZoom);
    if (pickerMap.getZoom() > targetMaxZoom) {
      pickerMap.setZoom(targetMaxZoom);
    }
  }

  async function openMapPicker() {
    const initialLat = parseCoordOrNull(fLatitude);
    const initialLng = parseCoordOrNull(fLongitude);
    const nextLat: number = typeof initialLat === 'number' && Number.isFinite(initialLat) ? initialLat : -6.2;
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
                  ? [`https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=${pickerMapTilerKey}`]
                  : ['https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'],
                tileSize: 256,
                attribution: pickerMapTilerKey ? 'MapTiler' : 'Esri',
                maxzoom: pickerSatelliteMaxZoom,
              },
            },
            layers: [
              { id: 'location-picker-standard', type: 'raster', source: 'standard' },
              {
                id: 'location-picker-satellite',
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
        pickerMap.addControl(new libregl.NavigationControl({ showCompass: true, showZoom: true }), 'top-right');
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
          Math.min(13, pickerViewMode === 'satellite' ? pickerSatelliteMaxZoom : pickerStandardMaxZoom),
        );
      }
      syncPickerViewMode();
      setPickerPoint(nextLat, nextLng);
    } catch (e: any) {
      pickerMapUnavailable = true;
      pickerMapErrorMessage = e?.message || 'Failed to initialize map';
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
      toast.error('Pilih titik lokasi terlebih dulu');
      return;
    }
    fLatitude = String(pickerLat);
    fLongitude = String(pickerLng);
    closeMapPicker();
  }
</script>

<div class="page-content fade-in">
  <div class="page-header">
    <div>
      <div class="kicker">
        <span class="dot"></span>
        {$t('dashboard.locations.kicker') || 'Customer portal'}
      </div>
      <h1>{$t('dashboard.locations.title') || 'My Locations'}</h1>
      <p class="subtitle">
        Kelola lokasi layanan Anda di sini. Saat membuat atau mengubah lokasi, titik map wajib dipilih.
      </p>
    </div>
    <div class="header-actions">
      <button class="btn-primary" onclick={openCreateLocation} disabled={loading || !hasLinkedCustomer}>
        <Icon name="plus" size={16} />
        Tambah Lokasi
      </button>
      <button class="btn-secondary" onclick={load} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </div>

  <div class="summary-grid">
    <div class="summary card">
      <div class="summary-label">Total lokasi</div>
      <div class="summary-value">{totalLocations}</div>
    </div>
    <div class="summary card">
      <div class="summary-label">Sudah pin map</div>
      <div class="summary-value">{mappedLocations}</div>
    </div>
    <div class="summary card">
      <div class="summary-label">Ada catatan</div>
      <div class="summary-value">{notedLocations}</div>
    </div>
  </div>

  {#if !hasLinkedCustomer}
    <div class="error-banner">
      <Icon name="alert-triangle" size={18} />
      <span>Akun ini belum terhubung ke customer, jadi lokasi layanan belum bisa dikelola.</span>
    </div>
  {/if}

  {#if error}
    <div class="error-banner">
      <Icon name="alert-triangle" size={18} />
      <span>{error}</span>
    </div>
  {/if}

  {#if loading}
    <div class="loading-card card">
      <div class="spinner"></div>
      <p>{$t('common.loading') || 'Loading...'}</p>
    </div>
  {:else if locations.length === 0}
    <div class="empty card">
      <Icon name="map-pin" size={28} />
      <div class="empty-text">
        <div class="title">Belum ada lokasi layanan.</div>
        <div class="sub">Tambahkan lokasi baru lalu pilih titik map agar bisa dipakai untuk order dan coverage check.</div>
      </div>
    </div>
  {:else}
    <div class="grid">
      {#each locations as loc (loc.id)}
        <div class="location card">
          <div class="top">
            <div class="badge">
              <Icon name="map-pin" size={16} />
              <span>Service Location</span>
            </div>
            <div class="row-actions">
              <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => openEditLocation(loc)}>
                <Icon name="edit" size={14} />
              </button>
              <button class="btn-icon danger" title={$t('common.delete') || 'Delete'} onclick={() => askDeleteLocation(loc.id)}>
                <Icon name="trash-2" size={14} />
              </button>
            </div>
          </div>
          <div class="name">{loc.label || 'Location'}</div>
          <div class="addr">{formatAddress(loc) || 'Alamat belum diisi'}</div>
          <div class="coords">
            {#if loc.latitude != null && loc.longitude != null}
              <span class="coord-chip">{Number(loc.latitude).toFixed(6)}, {Number(loc.longitude).toFixed(6)}</span>
            {:else}
              <span class="coord-chip missing">Belum ada titik map</span>
            {/if}
          </div>
          {#if loc.notes}
            <div class="notes">{loc.notes}</div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<Modal
  show={showLocationModal}
  width="760px"
  title={editingLocation ? 'Edit Lokasi' : 'Tambah Lokasi'}
  onclose={() => (showLocationModal = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>Label lokasi</span>
        <input class="input" bind:value={fLabel} placeholder="Contoh: Rumah, Kantor, Gudang" />
      </label>
      <label>
        <span>Negara</span>
        <select class="input" bind:value={fCountry}>
          <option value="ID">ID (Indonesia)</option>
          <option value="US">US (United States)</option>
        </select>
      </label>
    </div>

    <label>
      <span>Alamat line 1</span>
      <input class="input" bind:value={fLine1} placeholder="Jl. / street / building" />
    </label>

    <label>
      <span>Alamat line 2</span>
      <input class="input" bind:value={fLine2} placeholder="Blok, RT/RW, unit, lantai, dll" />
    </label>

    <div class="grid3">
      <label>
        <span>Kota</span>
        <input class="input" bind:value={fCity} />
      </label>
      <label>
        <span>Provinsi / State</span>
        <input class="input" bind:value={fState} />
      </label>
      <label>
        <span>Kode pos</span>
        <input class="input" bind:value={fPostal} />
      </label>
    </div>

    <label>
      <span>Catatan</span>
      <textarea class="input textarea" bind:value={fNotes} rows="3" placeholder="Catatan akses lokasi, patokan rumah, dll"></textarea>
    </label>

    <div class="map-picked-box">
      <div>
        <div class="map-picked-title">Titik lokasi di map</div>
        <div class="map-picked-sub">Wajib pilih titik map saat create atau edit.</div>
      </div>
      <button class="btn-secondary" type="button" onclick={openMapPicker}>
        <Icon name="map" size={16} />
        {fLatitude && fLongitude ? 'Ubah Titik Map' : 'Pilih Titik Map'}
      </button>
    </div>

    <div class="grid2">
      <label>
        <span>Latitude</span>
        <input class="input mono" bind:value={fLatitude} readonly />
      </label>
      <label>
        <span>Longitude</span>
        <input class="input mono" bind:value={fLongitude} readonly />
      </label>
    </div>

    <div class="modal-actions">
      <button class="btn-secondary" type="button" onclick={() => (showLocationModal = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn-primary"
        type="button"
        onclick={saveLocation}
        disabled={savingLocation || !fLabel.trim() || !fLatitude.trim() || !fLongitude.trim()}
      >
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<Modal show={showMapPicker} title="Pilih Titik Lokasi" width="860px" onclose={closeMapPicker}>
  <div class="map-picker-shell">
    <div class="map-picker-help">
      Klik peta untuk memilih titik. Setelah itu marker bisa di-drag untuk presisi.
    </div>
    <div class="map-picker-cords">
      {#if pickerLat != null && pickerLng != null}
        <span class="mono">{pickerLat.toFixed(7)}, {pickerLng.toFixed(7)}</span>
      {/if}
    </div>
    <MapCanvasShell
      bind:mapEl={pickerMapHost}
      bind:viewMode={pickerViewMode}
      on:searchselect={onPickerSearchSelect}
      loading={pickerMapLoading}
      mapUnavailable={pickerMapUnavailable}
      mapErrorMessage={pickerMapErrorMessage}
      mapUnavailableTitle="Map unavailable"
      mapUnavailableSubtitle="Unable to initialize WebGL map on this browser/device."
      height="min(58vh, 520px)"
    />
    <div class="modal-actions">
      <button class="btn-secondary" type="button" onclick={closeMapPicker}>Cancel</button>
      <button class="btn-primary" type="button" onclick={applyPickedCoordinates}>
        <Icon name="check" size={16} />
        Gunakan Titik Ini
      </button>
    </div>
  </div>
</Modal>

<ConfirmDialog
  show={showDeleteDialog}
  title={$t('common.delete') || 'Delete'}
  message="Lokasi ini akan dihapus dari akun customer. Lanjutkan?"
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={deletingLocation}
  onconfirm={doDeleteLocation}
  oncancel={() => (showDeleteDialog = false)}
/>

<style>
  .page-content {
    padding: 1.1rem 1.35rem 1.4rem;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.35rem;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: rgba(99, 102, 241, 0.9);
    box-shadow: 0 0 0 6px rgba(99, 102, 241, 0.12);
  }

  .subtitle {
    color: var(--text-secondary);
    margin-top: 0.35rem;
    max-width: 720px;
  }

  .header-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.8rem;
    margin-bottom: 0.95rem;
  }

  .summary {
    padding: 0.95rem 1rem;
  }

  .summary-label {
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 650;
  }

  .summary-value {
    margin-top: 0.3rem;
    font-size: 1.5rem;
    font-weight: 800;
    color: var(--text-primary);
  }

  .btn-primary,
  .btn-secondary {
    border-radius: 12px;
    padding: 0.55rem 0.85rem;
    border: 1px solid var(--border-color);
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    cursor: pointer;
    font-weight: 700;
  }

  .btn-primary {
    background: rgba(99, 102, 241, 0.95);
    border-color: rgba(99, 102, 241, 0.55);
    color: white;
  }

  .btn-secondary {
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .btn-primary:disabled,
  .btn-secondary:disabled,
  .btn-icon:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error-banner {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.75rem 0.9rem;
    border-radius: 12px;
    border: 1px solid rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.08);
    color: var(--text-primary);
    margin-bottom: 0.75rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
  }

  .location {
    padding: 1.15rem;
    position: relative;
    overflow: hidden;
  }

  .location::before {
    content: '';
    position: absolute;
    inset: -1px;
    background:
      radial-gradient(800px 240px at 0% 0%, rgba(99, 102, 241, 0.18), transparent 55%),
      radial-gradient(900px 260px at 100% 0%, rgba(34, 197, 94, 0.12), transparent 58%);
    pointer-events: none;
  }

  .top {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.9rem;
  }

  .row-actions {
    display: flex;
    gap: 0.45rem;
  }

  .btn-icon {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.35rem 0.45rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }

  .badge,
  .coord-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem;
    border-radius: 999px;
    border: 1px solid rgba(148, 163, 184, 0.35);
    background: rgba(148, 163, 184, 0.08);
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 650;
  }

  .coord-chip.missing {
    border-color: rgba(245, 158, 11, 0.4);
    background: rgba(245, 158, 11, 0.12);
  }

  .name,
  .addr,
  .coords,
  .notes {
    position: relative;
  }

  .name {
    font-size: 1.1rem;
    font-weight: 750;
    margin-bottom: 0.35rem;
  }

  .addr {
    color: var(--text-secondary);
    line-height: 1.4;
    font-size: 0.95rem;
  }

  .coords {
    margin-top: 0.85rem;
  }

  .notes {
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
    color: var(--text-secondary);
    font-size: 0.9rem;
    white-space: pre-wrap;
  }

  .empty {
    padding: 1.2rem;
    display: flex;
    gap: 0.9rem;
    align-items: flex-start;
  }

  .empty-text .title {
    font-weight: 750;
    margin-bottom: 0.25rem;
  }

  .empty-text .sub {
    color: var(--text-secondary);
  }

  .loading-card {
    padding: 1.25rem;
    display: grid;
    place-items: center;
    gap: 0.5rem;
  }

  .spinner {
    width: 26px;
    height: 26px;
    border-radius: 999px;
    border: 3px solid rgba(148, 163, 184, 0.3);
    border-top-color: rgba(99, 102, 241, 0.9);
    animation: spin 0.9s linear infinite;
  }

  .form {
    display: grid;
    gap: 0.8rem;
  }

  .grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.7rem;
  }

  .grid3 {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.7rem;
  }

  label > span {
    display: block;
    margin-bottom: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.86rem;
  }

  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.6rem 0.7rem;
    outline: none;
  }

  .textarea {
    resize: vertical;
    min-height: 90px;
  }

  .mono {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
  }

  .map-picked-box {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 0.85rem 0.9rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    background: rgba(148, 163, 184, 0.06);
  }

  .map-picked-title {
    font-weight: 700;
    color: var(--text-primary);
  }

  .map-picked-sub {
    margin-top: 0.2rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .map-picker-shell {
    display: grid;
    gap: 0.85rem;
  }

  .map-picker-help,
  .map-picker-cords {
    color: var(--text-secondary);
  }

  .modal-actions {
    margin-top: 0.3rem;
    display: flex;
    justify-content: flex-end;
    gap: 0.7rem;
  }

  @keyframes spin {
    from {
      transform: rotate(0);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 980px) {
    .page-content {
      padding: 0.95rem;
    }

    .summary-grid,
    .grid2,
    .grid3 {
      grid-template-columns: 1fr;
    }

    .page-header,
    .map-picked-box {
      flex-direction: column;
      align-items: stretch;
    }

    .header-actions {
      width: 100%;
      justify-content: stretch;
    }

    .header-actions > button {
      flex: 1 1 auto;
      justify-content: center;
    }
  }
</style>
