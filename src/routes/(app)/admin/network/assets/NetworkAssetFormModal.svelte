<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import {
    NETWORK_ASSET_STATUSES,
    NETWORK_ASSET_TYPES,
    getNetworkAssetTypeLabel
  } from '$lib/utils/networkAssetTypes';
  import {
    getNetworkAssetDetailFields,
    type NetworkAssetDetailDraft,
  } from '$lib/utils/networkAssetDetails';
  import type { NetworkAssetListItem } from '$lib/api/client';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type AssetDraft = {
    asset_type: string;
    name: string;
    code: string;
    vendor: string;
    model: string;
    serial_number: string;
    status: string;
    customer_id: string;
    location_id: string;
    work_order_id: string;
    parent_asset_id: string;
    latitude: string;
    longitude: string;
    notes: string;
  };

  type Props = {
    show: boolean;
    saving?: boolean;
    editing?: NetworkAssetListItem | null;
    draft: AssetDraft;
    detailDraft: NetworkAssetDetailDraft;
    onclearcoordinates?: () => void;
    onassettypechange?: (value: string) => void;
    onclose?: () => void;
    onsave?: () => void;
  };

  let {
    show = $bindable(false),
    saving = false,
    editing = null,
    draft,
    detailDraft,
    onclearcoordinates,
    onassettypechange,
    onclose,
    onsave,
  }: Props = $props();

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

  const detailFields = $derived.by(() => getNetworkAssetDetailFields(draft.asset_type));
  const coordinateLabel = $derived.by(() =>
    draft.latitude.trim() && draft.longitude.trim()
      ? `${Number(draft.latitude).toFixed(6)}, ${Number(draft.longitude).toFixed(6)}`
      : '',
  );
  const showMapPriorityHint = $derived.by(() =>
    ['odp', 'odc', 'fat', 'nap', 'olt', 'switch', 'router', 'odf', 'ups'].includes(draft.asset_type),
  );

  $effect(() => {
    pickerViewMode;
    if (!pickerMap) return;
    syncPickerViewMode();
  });

  onDestroy(() => {
    pickerMarker?.remove();
    pickerMap?.remove();
  });

  function parseCoordOrNull(v: string) {
    const raw = v.trim();
    if (!raw) return null;
    const parsed = Number(raw);
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
    const setVisibility = (layerId: string, visible: boolean) => {
      if (!pickerMap.getLayer(layerId)) return;
      pickerMap.setLayoutProperty(layerId, 'visibility', visible ? 'visible' : 'none');
    };
    setVisibility('asset-picker-standard', !showSatellite);
    setVisibility('asset-picker-satellite', showSatellite);
    const targetMaxZoom = showSatellite ? pickerSatelliteMaxZoom : pickerStandardMaxZoom;
    pickerMap.setMaxZoom(targetMaxZoom);
    if (pickerMap.getZoom() > targetMaxZoom) {
      pickerMap.setZoom(targetMaxZoom);
    }
  }

  async function openMapPicker() {
    const initialLat = parseCoordOrNull(draft.latitude) ?? -6.2;
    const initialLng = parseCoordOrNull(draft.longitude) ?? 106.816666;
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
              { id: 'asset-picker-standard', type: 'raster', source: 'standard' },
              {
                id: 'asset-picker-satellite',
                type: 'raster',
                source: 'satellite',
                layout: { visibility: 'none' },
              },
            ],
          },
          center: [initialLng, initialLat],
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
        pickerMap.setCenter([initialLng, initialLat]);
        pickerMap.setZoom(
          Math.min(
            13,
            pickerViewMode === 'satellite' ? pickerSatelliteMaxZoom : pickerStandardMaxZoom,
          ),
        );
      }
      syncPickerViewMode();
      setPickerPoint(initialLat, initialLng);
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
    if (pickerLat == null || pickerLng == null) return;
    draft.latitude = String(pickerLat);
    draft.longitude = String(pickerLng);
    closeMapPicker();
  }
</script>

<Modal
  bind:show
  title={
    editing
      ? $t('admin.ftth_assets.modal.title_edit') || 'Edit FTTH Asset'
      : $t('admin.ftth_assets.modal.title_create') || 'New FTTH Asset'
  }
  width="1120px"
  {onclose}
>
  <div class="modal-shell">
    <div class="form-column">
      <section class="panel-section full">
        <div class="section-head">
          <div>
            <span class="section-kicker">Asset Identity</span>
            <strong>{$t('admin.ftth_assets.modal.title_create') || 'Core Asset Data'}</strong>
          </div>
        </div>
        <div class="form-grid">
          <label>
            <span>{$t('admin.ftth_assets.fields.type') || 'Type'}</span>
            <select
              class="input"
              bind:value={draft.asset_type}
              disabled={saving}
              onchange={(event) => onassettypechange?.(event.currentTarget.value)}
            >
              {#each NETWORK_ASSET_TYPES as type}
                <option value={type}>{getNetworkAssetTypeLabel(type)}</option>
              {/each}
            </select>
          </label>

          <label>
            <span>{$t('admin.ftth_assets.fields.status') || 'Status'}</span>
            <select class="input" bind:value={draft.status} disabled={saving}>
              {#each NETWORK_ASSET_STATUSES as status}
                <option value={status}>{$t(`admin.ftth_assets.status.${status}`) || status}</option>
              {/each}
            </select>
          </label>

          <label class="full">
            <span>{$t('admin.ftth_assets.fields.name') || 'Name'}</span>
            <input
              class="input"
              bind:value={draft.name}
              disabled={saving}
              placeholder={$t('admin.ftth_assets.placeholders.name') || 'ODP-Cluster A'}
            />
          </label>

          <label>
            <span>{$t('admin.ftth_assets.fields.code') || 'Code'}</span>
            <input
              class="input"
              bind:value={draft.code}
              disabled={saving}
              placeholder={$t('admin.ftth_assets.placeholders.code') || 'ODP-001'}
            />
          </label>

          <label>
            <span>{$t('admin.ftth_assets.fields.serial_number') || 'Serial Number'}</span>
            <input
              class="input"
              bind:value={draft.serial_number}
              disabled={saving}
              placeholder={$t('admin.ftth_assets.placeholders.serial_number') || 'SN-123'}
            />
          </label>

          <label>
            <span>{$t('admin.ftth_assets.fields.vendor') || 'Vendor'}</span>
            <input
              class="input"
              bind:value={draft.vendor}
              disabled={saving}
              placeholder={$t('admin.ftth_assets.placeholders.vendor') || 'ZTE'}
            />
          </label>

          <label>
            <span>{$t('admin.ftth_assets.fields.model') || 'Model'}</span>
            <input
              class="input"
              bind:value={draft.model}
              disabled={saving}
              placeholder={$t('admin.ftth_assets.placeholders.model') || 'F670L'}
            />
          </label>
        </div>
      </section>

      {#if detailFields.length > 0}
        <section class="panel-section full detail-section">
          <div class="detail-section__head">
            <div>
              <span class="section-kicker">Asset Detail & Capacity</span>
              <strong>{$t('admin.ftth_assets.details.title') || 'Detail khusus asset'}</strong>
            </div>
            <p class="field-hint">
              {$t('admin.ftth_assets.details.subtitle') ||
                'Field ini menyesuaikan tipe asset yang sedang dipilih.'}
            </p>
          </div>
          <div class="detail-grid">
            {#each detailFields as field}
              <label>
                <span>{field.label}</span>
                <input
                  class="input"
                  bind:value={detailDraft[field.key]}
                  disabled={saving}
                  inputmode={field.inputMode || 'text'}
                  placeholder={field.placeholder}
                />
              </label>
            {/each}
          </div>
        </section>
      {/if}

      <section class="panel-section full">
        <div class="section-head">
          <div>
            <span class="section-kicker">Notes</span>
            <strong>{$t('admin.ftth_assets.fields.notes') || 'Notes'}</strong>
          </div>
        </div>
        <label class="full">
          <textarea class="input textarea" bind:value={draft.notes} disabled={saving}></textarea>
        </label>
      </section>
    </div>

    <aside class="map-column">
      <section class="panel-section map-section">
        <div class="section-head">
          <div>
            <span class="section-kicker">Map Location</span>
            <strong>Asset coordinates</strong>
          </div>
          {#if showMapPriorityHint}
            <span class="priority-chip">Recommended</span>
          {/if}
        </div>
        <p class="field-hint">
          Koordinat ini adalah posisi fisik asset dan akan dipakai untuk topology map. Tidak harus sama dengan lokasi customer.
        </p>

        <div class="map-preview-card">
          <div class="map-preview-title">
            <span>{coordinateLabel || 'No map point selected yet'}</span>
          </div>
          <div class="map-preview-body">
            <div class="map-preview-placeholder">
              <Icon name="map" size={18} />
              <span>Pick a standalone map point for this asset.</span>
            </div>
          </div>
        </div>

        <div class="coordinate-grid">
          <label>
            <span>Latitude</span>
            <input
              class="input mono-input"
              bind:value={draft.latitude}
              disabled={saving}
              inputmode="decimal"
              placeholder="-6.214620"
            />
          </label>
          <label>
            <span>Longitude</span>
            <input
              class="input mono-input"
              bind:value={draft.longitude}
              disabled={saving}
              inputmode="decimal"
              placeholder="106.845130"
            />
          </label>
        </div>

        <div class="map-actions">
          <button class="btn ghost" type="button" onclick={openMapPicker} disabled={saving}>
            <Icon name="map-pin" size={16} />
            {coordinateLabel ? 'Update Map Point' : 'Pick on Map'}
          </button>
          <button
            class="btn ghost danger-ghost"
            type="button"
            onclick={onclearcoordinates}
            disabled={saving || (!draft.latitude.trim() && !draft.longitude.trim())}
          >
            <Icon name="x" size={16} />
            Clear Point
          </button>
        </div>
      </section>
    </aside>
  </div>

  {#snippet footer()}
    <button class="btn ghost" type="button" onclick={onclose}>
      {$t('admin.ftth_assets.actions.cancel') || 'Cancel'}
    </button>
    <button class="btn" type="button" onclick={onsave} disabled={saving}>
      {saving
        ? $t('admin.ftth_assets.actions.saving') || 'Saving...'
        : editing
          ? $t('admin.ftth_assets.actions.save_changes') || 'Save changes'
          : $t('admin.ftth_assets.actions.create_asset') || 'Create asset'}
    </button>
  {/snippet}
</Modal>

<Modal show={showMapPicker} title="Pick Asset Location" width="860px" onclose={closeMapPicker}>
  <div class="map-picker-shell">
    <div class="map-picker-help">Klik peta untuk pilih titik, lalu drag marker jika perlu presisi.</div>
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
      mapUnavailableTitle="Map unavailable"
      mapUnavailableSubtitle="Unable to initialize WebGL map on this browser/device."
      height="min(58vh, 520px)"
    />
    <div class="picker-actions">
      <button class="btn ghost" type="button" onclick={closeMapPicker}>Cancel</button>
      <button class="btn" type="button" onclick={applyPickedCoordinates}>
        <Icon name="check" size={16} />
        Use This Point
      </button>
    </div>
  </div>
</Modal>

<style>
  .modal-shell {
    display: grid;
    grid-template-columns: minmax(0, 1.45fr) minmax(320px, 0.9fr);
    gap: 1rem;
    align-items: start;
  }
  .form-column {
    display: grid;
    gap: 1rem;
  }
  .map-column {
    position: sticky;
    top: 0;
  }
  .panel-section {
    display: grid;
    gap: 0.85rem;
    padding: 1.05rem;
    border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, #eadfcd 12%);
    border-radius: 20px;
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--bg-surface, #111827) 92%, #fff8ed 8%) 0%,
        color-mix(in srgb, var(--bg-surface, #111827) 96%, transparent) 100%
      );
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.035);
  }
  .section-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }
  .section-kicker {
    display: inline-block;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    margin-bottom: 0.2rem;
  }
  .priority-chip {
    display: inline-flex;
    align-items: center;
    padding: 0.28rem 0.55rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 600;
    background: color-mix(in srgb, var(--color-success-500, #059669) 14%, transparent);
    color: var(--color-success-700, #047857);
    border: 1px solid color-mix(in srgb, var(--color-success-500, #059669) 28%, transparent);
  }
  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
  }
  .full {
    grid-column: 1 / -1;
  }
  label {
    display: grid;
    gap: 0.45rem;
  }
  .input {
    min-height: 44px;
    background: color-mix(in srgb, var(--bg-surface, #111827) 84%, #fff 16%);
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, #e9dbc6 16%);
    border-radius: 14px;
    padding: 0.72rem 0.82rem;
    color: var(--text-primary);
    outline: none;
    transition: border-color 0.16s ease, box-shadow 0.16s ease, background 0.16s ease;
  }
  .input:focus {
    border-color: color-mix(in srgb, var(--color-primary, #6366f1) 55%, var(--border-color));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary, #6366f1) 16%, transparent);
  }
  textarea.input {
    min-height: 120px;
    resize: vertical;
  }
  .textarea {
    min-height: 120px;
  }
  .mono-input {
    font-family: var(--font-mono, monospace);
  }
  .field-hint {
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.35;
  }
  .detail-section {
    background: color-mix(in srgb, var(--bg-surface, #111827) 88%, #fff4e3 6%);
  }
  .detail-section__head {
    display: grid;
    gap: 0.3rem;
  }
  .detail-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.9rem;
  }
  @media (max-width: 860px) {
    .detail-grid,
    .form-grid {
      grid-template-columns: 1fr;
    }
  }
  .map-section {
    gap: 1rem;
  }
  .map-preview-card {
    border: 1px dashed color-mix(in srgb, var(--border-subtle) 82%, #deceb6 18%);
    border-radius: 18px;
    overflow: hidden;
    background:
      linear-gradient(
        160deg,
        color-mix(in srgb, var(--bg-surface, #0f172a) 80%, #f0e6d5 20%) 0%,
        color-mix(in srgb, var(--bg-surface, #0f172a) 91%, transparent) 100%
      );
  }
  .map-preview-title {
    padding: 0.78rem 0.92rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 88%, #e8dcc9 12%);
    font-weight: 600;
  }
  .map-preview-body {
    min-height: 140px;
    padding: 1rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .map-preview-placeholder {
    display: grid;
    gap: 0.5rem;
    justify-items: center;
    text-align: center;
    color: var(--text-secondary);
  }
  .coordinate-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.85rem;
  }
  .map-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.65rem;
  }
  .danger-ghost {
    color: var(--color-danger-700, #b91c1c);
  }
  .map-picker-shell {
    display: grid;
    gap: 0.85rem;
  }
  .map-picker-help,
  .map-picker-cords {
    color: var(--text-secondary);
  }
  .picker-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
  }
  .form-column .panel-section:first-child {
    padding-top: 1.15rem;
  }
  @media (max-width: 1080px) {
    .modal-shell {
      grid-template-columns: 1fr;
    }
    .map-column {
      position: static;
    }
  }
  @media (max-width: 720px) {
    .form-grid {
      grid-template-columns: 1fr;
    }
    .coordinate-grid,
    .detail-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
