<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { t } from 'svelte-i18n';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import DateTimeLocalInput from '$lib/components/ui/DateTimeLocalInput.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type RouterRow = {
    id: string;
    name: string;
  };

  let {
    show = $bindable(false),
    editing = null,
    formName = $bindable(''),
    formHost = $bindable(''),
    formPort = $bindable(8728),
    formUsername = $bindable(''),
    formPassword = $bindable(''),
    formLatitude = $bindable(''),
    formLongitude = $bindable(''),
    formEnabled = $bindable(true),
    formMaintenanceEnabled = $bindable(false),
    formMaintenanceUntilLocal = $bindable(''),
    formMaintenanceReason = $bindable(''),
    onSubmit,
  }: {
    show?: boolean;
    editing?: RouterRow | null;
    formName?: string;
    formHost?: string;
    formPort?: number;
    formUsername?: string;
    formPassword?: string;
    formLatitude?: string;
    formLongitude?: string;
    formEnabled?: boolean;
    formMaintenanceEnabled?: boolean;
    formMaintenanceUntilLocal?: string;
    formMaintenanceReason?: string;
    onSubmit: () => void;
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

  const modalTitle = $derived(
    editing
      ? `${$t('admin.network.routers.actions.edit') || 'Edit'}: ${editing.name}`
      : $t('admin.network.routers.actions.add') || 'Add Router',
  );

  onDestroy(() => {
    if (pickerMap) {
      pickerMap.remove();
      pickerMap = null;
      pickerMarker = null;
    }
  });

  function parseCoordOrNull(v: string) {
    const parsed = Number(v.trim());
    return Number.isFinite(parsed) ? parsed : null;
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
    const setVis = (layerId: string, visible: boolean) => {
      if (!pickerMap.getLayer(layerId)) return;
      pickerMap.setLayoutProperty(layerId, 'visibility', visible ? 'visible' : 'none');
    };
    setVis('picker-base-standard', !showSatellite);
    setVis('picker-base-satellite', showSatellite);
    const targetMaxZoom = showSatellite ? pickerSatelliteMaxZoom : pickerStandardMaxZoom;
    pickerMap.setMaxZoom(targetMaxZoom);
    if (pickerMap.getZoom() > targetMaxZoom) {
      pickerMap.setZoom(targetMaxZoom);
    }
  }

  async function openMapPicker() {
    const initialLat = parseCoordOrNull(formLatitude) ?? -6.2;
    const initialLng = parseCoordOrNull(formLongitude) ?? 106.816666;
    pickerLat = initialLat;
    pickerLng = initialLng;
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
              { id: 'picker-base-standard', type: 'raster', source: 'standard' },
              {
                id: 'picker-base-satellite',
                type: 'raster',
                source: 'satellite',
                layout: { visibility: 'none' },
              },
            ],
          },
          center: [initialLng, initialLat],
          zoom: 12,
          maxZoom: pickerStandardMaxZoom,
        });
        (pickerMap as any).libregl = libregl;
        pickerMap.addControl(
          new libregl.NavigationControl({ showCompass: true, showZoom: true }),
          'top-right',
        );
        pickerMap.addControl(
          new libregl.GeolocateControl({
            trackUserLocation: false,
            showAccuracyCircle: true,
          }),
          'top-right',
        );
        pickerMap.on('click', (ev: any) => {
          const { lat, lng } = ev.lngLat;
          setPickerPoint(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
        });
      } else {
        pickerMap.resize();
        pickerMap.setCenter([initialLng, initialLat]);
        pickerMap.setZoom(
          Math.min(
            12,
            pickerViewMode === 'satellite' ? pickerSatelliteMaxZoom : pickerStandardMaxZoom,
          ),
        );
      }
      syncPickerViewMode();
      setPickerPoint(initialLat, initialLng);
    } catch (error: any) {
      pickerMapUnavailable = true;
      pickerMapErrorMessage = error?.message || 'Failed to initialize map';
    } finally {
      pickerMapLoading = false;
    }
  }

  function closeMapPicker() {
    showMapPicker = false;
  }

  function applyPickedCoordinates() {
    if (pickerLat == null || pickerLng == null) return;
    formLatitude = String(pickerLat);
    formLongitude = String(pickerLng);
    closeMapPicker();
  }

  function onPickerSearchSelect(event: CustomEvent<{ lat: number; lng: number; label: string }>) {
    const { lat, lng } = event.detail;
    setPickerPoint(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
    if (!pickerMap) return;
    const currentZoom = Number.isFinite(pickerMap.getZoom()) ? pickerMap.getZoom() : 12;
    pickerMap.flyTo({
      center: [lng, lat],
      zoom: Math.max(currentZoom, 13),
      essential: true,
    });
  }

  $effect(() => {
    pickerViewMode;
    if (!pickerMap) return;
    syncPickerViewMode();
  });
</script>

<Modal bind:show title={modalTitle} width="520px" onclose={() => (show = false)}>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      onSubmit();
    }}
  >
    <label>
      <span>{$t('admin.network.routers.form.name') || 'Name'}</span>
      <input bind:value={formName} placeholder="e.g. POP Router 1" />
    </label>
    <label>
      <span>{$t('admin.network.routers.form.host') || 'Host'}</span>
      <input bind:value={formHost} placeholder="192.168.88.1" />
    </label>

    <div class="grid2">
      <label>
        <span>{$t('network.map.latitude') || 'Latitude'}</span>
        <input
          type="number"
          bind:value={formLatitude}
          step="any"
          min="-90"
          max="90"
          placeholder="-6.200000"
        />
      </label>
      <label>
        <span>{$t('network.map.longitude') || 'Longitude'}</span>
        <input
          type="number"
          bind:value={formLongitude}
          step="any"
          min="-180"
          max="180"
          placeholder="106.816666"
        />
      </label>
    </div>

    <div class="coord-actions">
      <button class="btn ghost" type="button" onclick={openMapPicker}>
        <Icon name="map-pin" size={16} />
        {$t('network.router.pick_location') || 'Pick on map'}
      </button>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.network.routers.form.port') || 'Port'}</span>
        <input type="number" bind:value={formPort} min="1" max="65535" />
      </label>
      <label>
        <span>{$t('admin.network.routers.form.username') || 'Username'}</span>
        <input bind:value={formUsername} placeholder="admin" />
      </label>
    </div>

    <label>
      <span>{$t('admin.network.routers.form.password') || 'Password'}</span>
      <input
        type="password"
        bind:value={formPassword}
        placeholder={editing ? 'Leave blank to keep current password' : ''}
      />
    </label>

    <label class="check">
      <input type="checkbox" bind:checked={formEnabled} />
      <span>{$t('admin.network.routers.form.enabled') || 'Enabled'}</span>
    </label>

    <div class="divider"></div>

    <label class="check">
      <input type="checkbox" bind:checked={formMaintenanceEnabled} />
      <span>{$t('admin.network.routers.form.maintenance') || 'Maintenance (mute alerts)'}</span>
    </label>

    {#if formMaintenanceEnabled}
      <DateTimeLocalInput
        label={$t('admin.network.routers.form.maintenance_until') || 'Maintenance until'}
        bind:value={formMaintenanceUntilLocal}
        placeholder="YYYY-MM-DD HH:mm"
      />
      <label>
        <span>{$t('admin.network.routers.form.maintenance_reason') || 'Reason (optional)'}</span>
        <input bind:value={formMaintenanceReason} placeholder="e.g. Upgrade firmware" />
      </label>
    {/if}

    <div class="modal-actions">
      <button class="btn ghost" type="button" onclick={() => (show = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn" type="submit">
        <Icon name="save" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </form>
</Modal>

<Modal show={showMapPicker} title={$t('network.router.pick_location') || 'Pick Router Location'} width="860px" onclose={closeMapPicker}>
  <div class="map-picker-shell">
    <div class="map-picker-help">{$t('network.router.click_map_to_select') || 'Klik peta untuk pilih titik, lalu drag marker jika perlu presisi.'}</div>
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
      mapUnavailableTitle={$t('network.map.map_unavailable_title') || 'Map unavailable'}
      mapUnavailableSubtitle={$t('network.map.map_unavailable_subtitle') || 'Unable to initialize WebGL map on this browser/device.'}
      height="min(58vh, 520px)"
    />
    <div class="modal-actions">
      <button class="btn ghost" type="button" onclick={closeMapPicker}>{$t('common.cancel') || 'Cancel'}</button>
      <button class="btn" type="button" onclick={applyPickedCoordinates}>
        <Icon name="check" size={16} />
        {$t('network.router.use_this_point') || 'Use this point'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: white;
    font-weight: 700;
    cursor: pointer;
    transition: transform 0.12s ease, filter 0.12s ease;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .divider {
    height: 1px;
    background: var(--border-subtle);
    margin: 2px 0;
  }

  .coord-actions {
    display: flex;
    justify-content: flex-end;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--text-secondary);
    font-weight: 700;
  }

  input[type='password'],
  input[type='number'],
  input {
    background: var(--bg-input, color-mix(in srgb, var(--bg-card), transparent 8%));
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 10px 12px;
    color: var(--text-primary);
    outline: none;
  }

  input:focus {
    border-color: rgba(99, 102, 241, 0.55);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.18);
  }

  .grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 10px;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 12px;
  }

  .map-picker-shell {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .map-picker-help {
    color: var(--text-secondary);
    font-size: 0.92rem;
  }

  .map-picker-cords {
    color: var(--text-primary);
    min-height: 1.2rem;
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    color: var(--text-primary);
  }

  @media (max-width: 768px) {
    .grid2 {
      grid-template-columns: 1fr;
    }
  }
</style>
