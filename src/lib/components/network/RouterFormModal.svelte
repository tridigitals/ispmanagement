<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { t } from 'svelte-i18n';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Field from '$lib/components/ds/Field.svelte';

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
      ? `${$t('admin.network.routers.actions.edit')}: ${editing.name}`
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

<Modal bind:show title={modalTitle} width="560px" onclose={() => (show = false)}>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      onSubmit();
    }}
  >
    <Field
      stacked
      id="rt-name"
      label={$t('admin.network.routers.form.name')}
      value={formName}
      placeholder="POP Router 1"
      onchange={(v) => (formName = v)}
    />
    <div class="grid2">
      <Field
        stacked
        id="rt-host"
        label={$t('admin.network.routers.form.host')}
        value={formHost}
        placeholder="192.168.88.1"
        onchange={(v) => (formHost = v)}
      />
      <Field
        stacked
        id="rt-port"
        label={$t('admin.network.routers.form.port')}
        value={String(formPort)}
        type="number"
        min={1}
        max={65535}
        onchange={(v) => (formPort = Number(v) || 0)}
      />
    </div>

    <Field
      stacked
      id="rt-user"
      label={$t('admin.network.routers.form.username')}
      value={formUsername}
      placeholder="admin"
      onchange={(v) => (formUsername = v)}
    />
    <Field
      stacked
      id="rt-pass"
      label={$t('admin.network.routers.form.password')}
      value={formPassword}
      type="password"
      placeholder={editing ? $t('admin.network.routers.form.password_keep') : ''}
      help={editing ? undefined : $t('admin.network.routers.form.password_help')}
      onchange={(v) => (formPassword = v)}
    />

    <div class="grid2">
      <Field
        stacked
        id="rt-lat"
        label={$t('network.map.latitude')}
        value={formLatitude}
        type="number"
        placeholder="-6.200000"
        onchange={(v) => (formLatitude = v)}
      />
      <Field
        stacked
        id="rt-lng"
        label={$t('network.map.longitude')}
        value={formLongitude}
        type="number"
        placeholder="106.816666"
        onchange={(v) => (formLongitude = v)}
      />
    </div>
    <div class="coord-actions">
      <Button variant="secondary" size="sm" icon="pin" type="button" onclick={openMapPicker}>
        {$t('network.router.pick_location')}
      </Button>
    </div>

    <Field
      stacked
      id="rt-enabled"
      label={$t('admin.network.routers.form.enabled')}
      value={formEnabled ? 'true' : 'false'}
      type="toggle"
      onchange={(v) => (formEnabled = v === 'true')}
    />

    <div class="divider"></div>

    <Field
      stacked
      id="rt-maint"
      label={$t('admin.network.routers.form.maintenance')}
      value={formMaintenanceEnabled ? 'true' : 'false'}
      type="toggle"
      help={$t('admin.network.routers.form.maintenance_help')}
      onchange={(v) => (formMaintenanceEnabled = v === 'true')}
    />

    {#if formMaintenanceEnabled}
      <DateTimeLocalInput
        label={$t('admin.network.routers.form.maintenance_until')}
        bind:value={formMaintenanceUntilLocal}
        placeholder="YYYY-MM-DD HH:mm"
      />
      <Field
        stacked
        id="rt-maint-reason"
        label={$t('admin.network.routers.form.maintenance_reason')}
        value={formMaintenanceReason}
        placeholder={$t('admin.network.routers.form.maintenance_reason_ph')}
        onchange={(v) => (formMaintenanceReason = v)}
      />
    {/if}

    <div class="modal-actions">
      <Button variant="ghost" type="button" onclick={() => (show = false)}>
        {$t('common.cancel')}
      </Button>
      <Button variant="primary" type="submit" icon="check">{$t('common.save')}</Button>
    </div>
  </form>
</Modal>

<Modal show={showMapPicker} title={$t('network.router.pick_location')} width="860px" onclose={closeMapPicker}>
  <div class="map-picker-shell">
    <div class="map-picker-help">{$t('network.router.click_map_to_select')}</div>
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
      mapUnavailableTitle={$t('network.map.map_unavailable_title')}
      mapUnavailableSubtitle={$t('network.map.map_unavailable_subtitle')}
      height="min(58vh, 520px)"
    />
    <div class="modal-actions">
      <Button variant="ghost" type="button" onclick={closeMapPicker}>{$t('common.cancel')}</Button>
      <Button variant="primary" type="button" icon="check" onclick={applyPickedCoordinates}>
        {$t('network.router.use_this_point')}
      </Button>
    </div>
  </div>
</Modal>

<style>
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

  .grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
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
