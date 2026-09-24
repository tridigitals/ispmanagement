<script lang="ts">
  /**
   * CoordMapPicker — pemilih koordinat berbasis peta (inline, bukan modal).
   *
   * Kontrak data: menulis ke state `latitude`/`longitude` (string) yang sama
   * seperti input manual — nama & tipe TIDAK berubah.
   *
   * Dipakai oleh: RouterAddWizard step Lokasi (v2) dan bisa dipakai form lain
   * yang butuh pilih titik di peta. Pakai `MapCanvasShell` yang sudah ada,
   * maplibre-gl di-lazy-import, dan teardown saat unmount untuk hindari
   * kebocoran WebGL context.
   */
  import { onDestroy, onMount, tick } from 'svelte';
  import { t } from 'svelte-i18n';
  import Button from '$lib/components/ds/Button.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';

  let {
    latitude = $bindable(''),
    longitude = $bindable(''),
    height = 'min(48vh, 420px)',
  }: {
    latitude?: string;
    longitude?: string;
    height?: string;
  } = $props();

  const MAPTILER_KEY = (import.meta.env.VITE_MAPTILER_KEY as string | undefined)?.trim();
  const STANDARD_MAX_ZOOM = 19;
  const SATELLITE_MAX_ZOOM = MAPTILER_KEY ? 21 : 18;

  let mapHost = $state<HTMLDivElement | null>(null);
  let viewMode = $state<'standard' | 'satellite'>('standard');
  let loading = $state(false);
  let unavailable = $state(false);
  let errorMessage = $state('');

  let map: any = null;
  let marker: any = null;
  let maplibrePromise: Promise<any> | null = null;
  let pickLat = $state<number | null>(null);
  let pickLng = $state<number | null>(null);

  const hasPoint = $derived(pickLat != null && pickLng != null);

  function coordOrNull(v: string) {
    const raw = (v ?? '').trim();
    if (!raw) return null; /* '' → Number('') = 0, bukan null — wajib dicek dulu */
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : null;
  }

  async function getMaplibre() {
    if (!maplibrePromise) maplibrePromise = import('maplibre-gl');
    return maplibrePromise;
  }

  /* Satu-satunya penulis ke state form — nama field dipertahankan. */
  function writeLatLng(lat: number, lng: number) {
    pickLat = lat;
    pickLng = lng;
    latitude = lat.toFixed(7);
    longitude = lng.toFixed(7);
  }

  function placeMarker(lat: number, lng: number) {
    if (!map) return;
    if (marker) {
      marker.setLngLat([lng, lat]);
      return;
    }
    marker = new (map as any).libregl.Marker({ draggable: true })
      .setLngLat([lng, lat])
      .addTo(map);
    marker.on('dragend', () => {
      const pos = marker.getLngLat();
      writeLatLng(Number(pos.lat.toFixed(7)), Number(pos.lng.toFixed(7)));
    });
  }

  function syncViewMode() {
    if (!map) return;
    const sat = viewMode === 'satellite';
    const setVis = (id: string, visible: boolean) => {
      if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', visible ? 'visible' : 'none');
    };
    setVis('cp-base-standard', !sat);
    setVis('cp-base-satellite', sat);
    const maxZoom = sat ? SATELLITE_MAX_ZOOM : STANDARD_MAX_ZOOM;
    map.setMaxZoom(maxZoom);
    if (map.getZoom() > maxZoom) map.setZoom(maxZoom);
  }

  async function ensureMap() {
    if (map) return;
    await tick();
    if (!mapHost) {
      unavailable = true;
      errorMessage = $t('network.map.picker_container_missing');
      return;
    }
    loading = true;
    try {
      const libregl = await getMaplibre();
      const startLat = coordOrNull(latitude) ?? -6.2;
      const startLng = coordOrNull(longitude) ?? 106.816666;
      map = new libregl.Map({
        container: mapHost,
        style: {
          version: 8,
          sources: {
            standard: {
              type: 'raster',
              tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
              tileSize: 256,
              attribution: '© OpenStreetMap contributors',
              maxzoom: STANDARD_MAX_ZOOM,
            },
            satellite: {
              type: 'raster',
              tiles: MAPTILER_KEY
                ? [`https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=${MAPTILER_KEY}`]
                : [
                    'https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
                  ],
              tileSize: 256,
              attribution: MAPTILER_KEY ? 'MapTiler' : 'Esri',
              maxzoom: SATELLITE_MAX_ZOOM,
            },
          },
          layers: [
            { id: 'cp-base-standard', type: 'raster', source: 'standard' },
            { id: 'cp-base-satellite', type: 'raster', source: 'satellite', layout: { visibility: 'none' } },
          ],
        },
        center: [startLng, startLat],
        zoom: 12,
        maxZoom: STANDARD_MAX_ZOOM,
      });
      (map as any).libregl = libregl;
      map.addControl(new libregl.NavigationControl({ showCompass: true, showZoom: true }), 'top-right');
      map.addControl(
        new libregl.GeolocateControl({ trackUserLocation: false, showAccuracyCircle: true }),
        'top-right',
      );
      map.on('click', (ev: any) => {
        writeLatLng(Number(ev.lngLat.lat.toFixed(7)), Number(ev.lngLat.lng.toFixed(7)));
        placeMarker(pickLat as number, pickLng as number);
      });
      syncViewMode();
      pickLat = coordOrNull(latitude);
      pickLng = coordOrNull(longitude);
      if (hasPoint) placeMarker(pickLat as number, pickLng as number);
    } catch (err: any) {
      unavailable = true;
      errorMessage = err?.message || '';
    } finally {
      loading = false;
    }
  }

  function onSearchSelect(event: CustomEvent<{ lat: number; lng: number }>) {
    const { lat, lng } = event.detail;
    writeLatLng(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
    placeMarker(pickLat as number, pickLng as number);
    if (!map) return;
    const zoom = Number.isFinite(map.getZoom()) ? map.getZoom() : 12;
    map.flyTo({ center: [lng, lat], zoom: Math.max(zoom, 13), essential: true });
  }

  function clearPoint() {
    pickLat = null;
    pickLng = null;
    latitude = '';
    longitude = '';
    if (marker) {
      marker.remove();
      marker = null;
    }
  }

  /* Teardown wajib: hindari WebGL context bocor saat step berpindah/tutup. */
  export function teardown() {
    try {
      marker?.remove();
    } catch {}
    try {
      map?.remove();
    } catch {}
    marker = null;
    map = null;
  }

  export function ensureReady() {
    void ensureMap();
  }

  /* Inline picker: langsung siapkan peta saat komponen mount. */
  onMount(() => {
    void ensureMap();
  });

  onDestroy(teardown);
</script>

<div class="picker">
  <div class="picker-bar">
    <span class="picker-hint">{$t('network.map.picker_hint')}</span>
    <div class="picker-cords">
      {#if hasPoint}
        <span class="mono">{(pickLat as number).toFixed(7)}, {(pickLng as number).toFixed(7)}</span>
      {:else}
        <span class="picker-empty">{$t('network.map.picker_no_point')}</span>
      {/if}
      {#if hasPoint}
        <Button variant="ghost" size="sm" icon="close" type="button" onclick={clearPoint}>
          {$t('common.clear')}
        </Button>
      {/if}
    </div>
  </div>

  <MapCanvasShell
    bind:mapEl={mapHost}
    bind:viewMode
    on:searchselect={onSearchSelect}
    {loading}
    mapUnavailable={unavailable}
    mapErrorMessage={errorMessage}
    mapUnavailableTitle={$t('network.map.map_unavailable_title')}
    mapUnavailableSubtitle={$t('network.map.map_unavailable_subtitle')}
    {height}
  />
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .picker-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .picker-hint {
    color: var(--text-secondary);
    font-size: 0.86rem;
  }

  .picker-cords {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 1.4rem;
  }

  .picker-empty {
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    font-size: 0.82rem;
    color: var(--text-primary);
  }
</style>
