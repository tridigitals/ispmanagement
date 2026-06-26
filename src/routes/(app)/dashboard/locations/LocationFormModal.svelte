<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { t } from 'svelte-i18n';
  import { toast } from 'svelte-sonner';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type CustomerLocation = {
    id: string;
    label?: string | null;
  };

  let {
    show = $bindable(false),
    editingLocation = null,
    savingLocation = false,
    fLabel = $bindable(''),
    fLine1 = $bindable(''),
    fLine2 = $bindable(''),
    fCity = $bindable(''),
    fState = $bindable(''),
    fPostal = $bindable(''),
    fCountry = $bindable('ID'),
    fNotes = $bindable(''),
    fLatitude = $bindable(''),
    fLongitude = $bindable(''),
    onSave,
  }: {
    show?: boolean;
    editingLocation?: CustomerLocation | null;
    savingLocation?: boolean;
    fLabel?: string;
    fLine1?: string;
    fLine2?: string;
    fCity?: string;
    fState?: string;
    fPostal?: string;
    fCountry?: string;
    fNotes?: string;
    fLatitude?: string;
    fLongitude?: string;
    onSave: () => void;
  } = $props();

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

  const modalTitle = $derived(editingLocation ? 'Edit Lokasi' : 'Tambah Lokasi');

  onDestroy(() => {
    pickerMarker?.remove();
    pickerMap?.remove();
  });

  function parseCoordOrNull(v: string) {
    const raw = v.trim();
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
    pickerMap?.flyTo({
      center: [lng, lat],
      zoom: Math.max(pickerMap.getZoom(), 15),
      duration: 480,
    });
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

  $effect(() => {
    pickerViewMode;
    if (!pickerMap) return;
    syncPickerViewMode();
  });
</script>

<Modal bind:show width="760px" title={modalTitle} onclose={() => (show = false)}>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('dashboard.locations.form.label')}</span>
        <input class="input" bind:value={fLabel} placeholder="Contoh: Rumah, Kantor, Gudang" />
      </label>
      <label>
        <span>{$t('dashboard.locations.form.country')}</span>
        <select class="input" bind:value={fCountry}>
          <option value="ID">{$t('profile.addresses.fields.country_id') || 'ID (Indonesia)'}</option>
          <option value="US">{$t('profile.addresses.fields.country_us') || 'US (United States)'}</option>
        </select>
      </label>
    </div>

    <label>
      <span>{$t('dashboard.locations.form.line1')}</span>
      <input class="input" bind:value={fLine1} placeholder="Jl. / street / building" />
    </label>

    <label>
      <span>{$t('dashboard.locations.form.line2')}</span>
      <input class="input" bind:value={fLine2} placeholder="Blok, RT/RW, unit, lantai, dll" />
    </label>

    <div class="grid3">
      <label>
        <span>{$t('dashboard.locations.form.city')}</span>
        <input class="input" bind:value={fCity} />
      </label>
      <label>
        <span>Provinsi / State</span>
        <input class="input" bind:value={fState} />
      </label>
      <label>
        <span>{$t('dashboard.locations.form.postal_code')}</span>
        <input class="input" bind:value={fPostal} />
      </label>
    </div>

    <label>
      <span>{$t('dashboard.locations.form.notes')}</span>
      <textarea
        class="input textarea"
        bind:value={fNotes}
        rows="3"
        placeholder={$t('dashboard.locations.form.notes_placeholder')}
      ></textarea>
    </label>

    <div class="map-picked-box">
      <div>
        <div class="map-picked-title">{$t('dashboard.locations.form.pick_on_map')}</div>
        <div class="map-picked-sub">{$t('dashboard.locations.form.no_map_pin')}</div>
      </div>
      <button class="btn-secondary" type="button" onclick={openMapPicker}>
        <Icon name="map" size={16} />
        {$t('dashboard.locations.form.pick_on_map')}
      </button>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('common.latitude') || 'Latitude'}</span>
        <input class="input mono" bind:value={fLatitude} readonly />
      </label>
      <label>
        <span>{$t('common.longitude') || 'Longitude'}</span>
        <input class="input mono" bind:value={fLongitude} readonly />
      </label>
    </div>

    <div class="modal-actions">
      <button class="btn-secondary" type="button" onclick={() => (show = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn-primary"
        type="button"
        onclick={onSave}
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

<style>
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
    background: var(--bg-tertiary);
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
    background: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-color));
    color: var(--bg-app);
  }

  .btn-secondary {
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .btn-primary:disabled,
  .btn-secondary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  @media (max-width: 980px) {
    .grid2,
    .grid3 {
      grid-template-columns: 1fr;
    }

    .map-picked-box {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
