<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import DateTimeLocalInput from '$lib/components/ui/DateTimeLocalInput.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type RouterRow = {
    id: string;
    name: string;
    host: string;
    port: number;
    username: string;
    enabled: boolean;
    identity?: string | null;
    ros_version?: string | null;
    is_online: boolean;
    last_seen_at?: string | null;
    latency_ms?: number | null;
    last_error?: string | null;
    maintenance_until?: string | null;
    maintenance_reason?: string | null;
    latitude?: number | null;
    longitude?: number | null;
    updated_at?: string;
  };

  let loading = $state(true);
  let routers = $state<RouterRow[]>([]);
  let search = $state('');
  let refreshing = $state(false);
  let lastRefreshAt = $state<number | null>(null);
  let isMobile = $state(false);

  let showModal = $state(false);
  let editing: RouterRow | null = $state(null);

  let formName = $state('');
  let formHost = $state('');
  let formPort = $state(8728);
  let formUsername = $state('');
  let formPassword = $state('');
  let formLatitude = $state('');
  let formLongitude = $state('');
  let formEnabled = $state(true);
  let formMaintenanceEnabled = $state(false);
  let formMaintenanceUntilLocal = $state('');
  let formMaintenanceReason = $state('');
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

  function isoToLocalInput(iso?: string | null) {
    if (!iso) return '';
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    const pad = (n: number) => String(n).padStart(2, '0');
    const yyyy = d.getFullYear();
    const mm = pad(d.getMonth() + 1);
    const dd = pad(d.getDate());
    const hh = pad(d.getHours());
    const mi = pad(d.getMinutes());
    return `${yyyy}-${mm}-${dd}T${hh}:${mi}`;
  }

  function localInputToIso(local: string): string | null {
    if (!local) return null;
    const d = new Date(local);
    if (Number.isNaN(d.getTime())) return null;
    return d.toISOString();
  }

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return routers;
    return routers.filter((r) => {
      const hay = `${r.name} ${r.host} ${r.identity || ''}`.toLowerCase();
      return hay.includes(q);
    });
  });

  const stats = $derived.by(() => {
    const total = routers.length;
    const online = routers.filter((r) => r.is_online).length;
    const offline = total - online;
    return { total, online, offline };
  });

  const columns = $derived.by(() => [
    { key: 'name', label: $t('admin.network.routers.columns.name') || 'Name' },
    { key: 'host', label: $t('admin.network.routers.columns.host') || 'Host' },
    { key: 'status', label: $t('admin.network.routers.columns.status') || 'Status' },
    { key: 'latency', label: $t('admin.network.routers.columns.latency') || 'Latency' },
    { key: 'seen', label: $t('admin.network.routers.columns.seen') || 'Last Seen' },
    { key: 'actions', label: '', align: 'right' as const, width: '220px' },
  ]);

  let refreshHandle: any = null;

  onMount(() => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 1024px)');
      const sync = () => (isMobile = mq.matches);
      sync();
      try {
        mq.addEventListener('change', sync);
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
      }
    }

    void load();

    // Keep status reasonably fresh without requiring manual refresh.
    // Note: server also runs a background poller; this is just UI sync.
    const intervalMs = 5000;
    refreshHandle = setInterval(() => {
      void refreshSilent();
    }, intervalMs);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
    if (pickerMap) {
      pickerMap.remove();
      pickerMap = null;
      pickerMarker = null;
    }
  });

  async function load() {
    loading = true;
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      lastRefreshAt = Date.now();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function refreshSilent() {
    if (refreshing || showModal) return;
    refreshing = true;
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      lastRefreshAt = Date.now();
    } catch {
      // ignore (avoid noisy toasts for background refresh)
    } finally {
      refreshing = false;
    }
  }

  function openCreate() {
    editing = null;
    formName = '';
    formHost = '';
    formPort = 8728;
    formUsername = '';
    formPassword = '';
    formLatitude = '';
    formLongitude = '';
    formEnabled = true;
    formMaintenanceEnabled = false;
    formMaintenanceUntilLocal = '';
    formMaintenanceReason = '';
    showModal = true;
  }

  function openEdit(r: RouterRow) {
    editing = r;
    formName = r.name || '';
    formHost = r.host || '';
    formPort = Number(r.port || 8728);
    formUsername = r.username || '';
    formPassword = '';
    formLatitude = r.latitude != null ? String(r.latitude) : '';
    formLongitude = r.longitude != null ? String(r.longitude) : '';
    formEnabled = r.enabled ?? true;
    const untilIso = r.maintenance_until ?? null;
    const untilMs = untilIso ? new Date(untilIso).getTime() : NaN;
    formMaintenanceEnabled = Number.isFinite(untilMs) ? untilMs > Date.now() : false;
    formMaintenanceUntilLocal = isoToLocalInput(untilIso);
    formMaintenanceReason = r.maintenance_reason || '';
    showModal = true;
  }

  async function save() {
    const name = formName.trim();
    const host = formHost.trim();
    if (!name || !host || !formUsername.trim()) {
      toast.error($t('common.validation_error') || 'Please fill required fields.');
      return;
    }
    if (!editing && !formPassword.trim()) {
      toast.error($t('admin.network.routers.form.password') || 'Password is required.');
      return;
    }
    const latRaw = formLatitude.trim();
    const lngRaw = formLongitude.trim();
    const parsedLat = latRaw ? Number(latRaw) : NaN;
    const parsedLng = lngRaw ? Number(lngRaw) : NaN;
    if (latRaw && (Number.isNaN(parsedLat) || parsedLat < -90 || parsedLat > 90)) {
      toast.error('Latitude must be between -90 and 90');
      return;
    }
    if (lngRaw && (Number.isNaN(parsedLng) || parsedLng < -180 || parsedLng > 180)) {
      toast.error('Longitude must be between -180 and 180');
      return;
    }
    const latitude = latRaw ? parsedLat : null;
    const longitude = lngRaw ? parsedLng : null;

    try {
      const maintenance_until = formMaintenanceEnabled
        ? localInputToIso(formMaintenanceUntilLocal)
        : null;
      const maintenance_reason = formMaintenanceEnabled ? formMaintenanceReason.trim() || null : null;

      if (editing) {
        await api.mikrotik.routers.update(editing.id, {
          name,
          host,
          port: formPort,
          username: formUsername.trim(),
          password: formPassword.trim() ? formPassword : undefined,
          enabled: formEnabled,
          maintenance_until,
          maintenance_reason,
          latitude,
          longitude,
        });
        toast.success($t('admin.network.routers.toasts.updated') || 'Router updated');
      } else {
        await api.mikrotik.routers.create({
          name,
          host,
          port: formPort,
          username: formUsername.trim(),
          password: formPassword,
          enabled: formEnabled,
          maintenance_until,
          maintenance_reason,
          latitude,
          longitude,
        });
        toast.success($t('admin.network.routers.toasts.created') || 'Router created');
      }
      showModal = false;
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function test(r: RouterRow) {
    try {
      const res = await api.mikrotik.routers.test(r.id);
      if (res?.ok) {
        toast.success(
          `${res.identity || r.name} • RouterOS ${res.ros_version || ''} • ${res.latency_ms ?? ''}ms`,
        );
      } else {
        toast.error(res?.error || 'Failed to connect');
      }
      await refreshSilent();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function remove(r: RouterRow) {
    const ok = confirm(`${$t('common.delete') || 'Delete'}: ${r.name}?`);
    if (!ok) return;
    try {
      await api.mikrotik.routers.delete(r.id);
      toast.success($t('admin.network.routers.toasts.deleted') || 'Router deleted');
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function openDetail(r: RouterRow) {
    goto(`${$page.url.pathname}/${r.id}`);
  }

  function statusLabel(r: RouterRow) {
    if (r.is_online) return $t('admin.network.routers.badges.online') || 'Online';
    return $t('admin.network.routers.badges.offline') || 'Offline';
  }

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
                  ? [`https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=${pickerMapTilerKey}`]
                  : ['https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'],
                tileSize: 256,
                attribution: pickerMapTilerKey ? 'MapTiler' : 'Esri',
                maxzoom: pickerSatelliteMaxZoom,
              },
            },
            layers: [
              { id: 'picker-base-standard', type: 'raster', source: 'standard' },
              { id: 'picker-base-satellite', type: 'raster', source: 'satellite', layout: { visibility: 'none' } },
            ],
          },
          center: [initialLng, initialLat],
          zoom: 12,
          maxZoom: pickerStandardMaxZoom,
        });
        (pickerMap as any).libregl = libregl;
        pickerMap.addControl(new libregl.NavigationControl({ showCompass: true, showZoom: true }), 'top-right');
        pickerMap.addControl(new libregl.GeolocateControl({ trackUserLocation: false, showAccuracyCircle: true }), 'top-right');
        pickerMap.on('click', (ev: any) => {
          const { lat, lng } = ev.lngLat;
          setPickerPoint(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
        });
      } else {
        pickerMap.resize();
        pickerMap.setCenter([initialLng, initialLat]);
        pickerMap.setZoom(Math.min(12, pickerViewMode === 'satellite' ? pickerSatelliteMaxZoom : pickerStandardMaxZoom));
      }
      syncPickerViewMode();
      setPickerPoint(initialLat, initialLng);
    } catch (error: any) {
      pickerMapUnavailable = true;
      pickerMapErrorMessage = error?.message || 'Failed to initialize map';
      toast.error(pickerMapErrorMessage);
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

<div class="page-content fade-in">
  <NetworkPageHeader
    title={$t('admin.network.routers.title') || 'Routers'}
    subtitle={$t('admin.network.routers.subtitle') || 'Manage MikroTik routers and monitoring'}
  >
    {#snippet actions()}
      <button class="btn ghost" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
        <Icon name="refresh-cw" size={16} />
        {$t('admin.network.routers.actions.refresh') || $t('common.refresh') || 'Refresh'}
      </button>

      {#if $can('manage', 'network_routers')}
        <button class="btn" type="button" onclick={openCreate}>
          <Icon name="plus" size={16} />
          {$t('admin.network.routers.actions.add') || 'Add Router'}
        </button>
      {/if}
    {/snippet}
  </NetworkPageHeader>

  <div class="stats">
    <div class="stat-card">
      <div class="stat-top">
        <span class="stat-label">Total</span>
        <Icon name="list" size={14} />
      </div>
      <div class="stat-value">{stats.total}</div>
    </div>
    <div class="stat-card tone-ok">
      <div class="stat-top">
        <span class="stat-label">Online</span>
        <Icon name="check-circle" size={14} />
      </div>
      <div class="stat-value">{stats.online}</div>
    </div>
    <div class="stat-card tone-bad">
      <div class="stat-top">
        <span class="stat-label">Offline</span>
        <Icon name="alert-circle" size={14} />
      </div>
      <div class="stat-value">{stats.offline}</div>
    </div>
  </div>

  <div class="toolbar">
    <div class="search">
      <Icon name="search" size={16} />
      <input
        class="search-input"
        bind:value={search}
        placeholder={$t('admin.network.routers.search') || 'Search routers...'}
      />
      {#if search}
        <button class="clear" type="button" onclick={() => (search = '')}>
          <Icon name="x" size={14} />
        </button>
      {/if}
    </div>
  </div>

  <div class="table-wrap">
    <Table
      {columns}
      data={filtered}
      loading={loading}
      emptyText={$t('admin.network.routers.empty') || 'No routers yet'}
      mobileView={isMobile ? 'card' : 'scroll'}
    >
      {#snippet cell({ item, key }: any)}
        {#if key === 'name'}
          <div class="name-cell">
            <div class="name-top">
              <span class="name">{item.name}</span>
              {#if item.identity}
                <span class="chip">{item.identity}</span>
              {/if}
              {#if item.maintenance_until && new Date(item.maintenance_until).getTime() > Date.now()}
                <span class="chip warn" title={item.maintenance_reason || ''}>
                  {$t('admin.network.routers.badges.maintenance') || 'Maintenance'}
                </span>
              {/if}
            </div>
            <div class="muted">{item.username}@{item.host}:{item.port}</div>
            {#if item.latitude != null && item.longitude != null}
              <div class="muted mono">{item.latitude.toFixed(6)}, {item.longitude.toFixed(6)}</div>
            {/if}
            {#if item.last_error}
              <div class="error">{item.last_error}</div>
            {/if}
          </div>
        {:else if key === 'host'}
          <span class="mono">{item.host}:{item.port}</span>
        {:else if key === 'status'}
          <span class="badge" class:online={item.is_online} class:offline={!item.is_online}>
            {statusLabel(item)}
          </span>
        {:else if key === 'latency'}
          {#if item.latency_ms != null}
            <span class="mono">{item.latency_ms} ms</span>
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if key === 'seen'}
          {#if item.last_seen_at}
            <span
              class="muted"
              title={formatDateTime(item.last_seen_at, { timeZone: $appSettings.app_timezone })}
              >{timeAgo(item.last_seen_at)}</span
            >
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if key === 'actions'}
          <div class="actions">
            <button class="icon-btn" type="button" onclick={() => openDetail(item)} title={$t('admin.network.routers.actions.open') || 'Open'}>
              <Icon name="arrow-right" size={16} />
            </button>
            <button class="icon-btn" type="button" onclick={() => test(item)} title={$t('admin.network.routers.actions.test') || 'Test Connection'}>
              <Icon name="zap" size={16} />
            </button>
            {#if $can('manage', 'network_routers')}
              <button class="icon-btn" type="button" onclick={() => openEdit(item)} title={$t('admin.network.routers.actions.edit') || 'Edit'}>
                <Icon name="edit" size={16} />
              </button>
              <button class="icon-btn danger" type="button" onclick={() => remove(item)} title={$t('admin.network.routers.actions.delete') || 'Delete'}>
                <Icon name="trash-2" size={16} />
              </button>
            {/if}
          </div>
        {/if}
      {/snippet}
    </Table>
  </div>
</div>

<Modal
  show={showModal}
  title={editing
    ? `${$t('admin.network.routers.actions.edit') || 'Edit'}: ${editing.name}`
    : $t('admin.network.routers.actions.add') || 'Add Router'}
  width="520px"
  onclose={() => (showModal = false)}
>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      void save();
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
        <span>Latitude</span>
        <input type="number" bind:value={formLatitude} step="any" min="-90" max="90" placeholder="-6.200000" />
      </label>
      <label>
        <span>Longitude</span>
        <input type="number" bind:value={formLongitude} step="any" min="-180" max="180" placeholder="106.816666" />
      </label>
    </div>
    <div class="coord-actions">
      <button class="btn ghost" type="button" onclick={openMapPicker}>
        <Icon name="map-pin" size={16} />
        Pick on map
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
      <button class="btn ghost" type="button" onclick={() => (showModal = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn" type="submit">
        <Icon name="save" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </form>
</Modal>

<Modal show={showMapPicker} title="Pick Router Location" width="860px" onclose={closeMapPicker}>
  <div class="map-picker-shell">
    <div class="map-picker-help">Klik peta untuk pilih titik, lalu drag marker jika perlu presisi.</div>
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
      <button class="btn ghost" type="button" onclick={closeMapPicker}>Cancel</button>
      <button class="btn" type="button" onclick={applyPickedCoordinates}>
        <Icon name="check" size={16} />
        Use this point
      </button>
    </div>
  </div>
</Modal>

<style>
  .page-content {
    padding: 28px;
  }

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

  .btn:hover {
    filter: brightness(1.05);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }

  .stat-card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 14px 14px 12px;
  }

  .stat-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-secondary);
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .stat-value {
    margin-top: 10px;
    font-size: 1.6rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .tone-ok {
    box-shadow: 0 0 0 1px rgba(34, 197, 94, 0.15) inset;
  }

  .tone-bad {
    box-shadow: 0 0 0 1px rgba(239, 68, 68, 0.16) inset;
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px 12px;
    min-width: min(560px, 100%);
    color: var(--text-secondary);
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.95rem;
  }

  .clear {
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--text-secondary);
    display: grid;
    place-items: center;
  }

  .table-wrap {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
  }

  .name-cell .name-top {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .name {
    font-weight: 900;
    color: var(--text-primary);
  }

  .chip {
    font-size: 0.72rem;
    font-weight: 800;
    padding: 3px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-hover), transparent 20%);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  .chip.warn {
    border-color: rgba(245, 158, 11, 0.28);
    background: rgba(245, 158, 11, 0.12);
    color: rgba(245, 158, 11, 0.95);
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
    color: var(--text-primary);
  }

  .muted {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .error {
    margin-top: 6px;
    color: color-mix(in srgb, #ef4444, var(--text-primary) 15%);
    font-size: 0.85rem;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.78rem;
    border: 1px solid var(--border-color);
  }

  .badge.online {
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
    border-color: rgba(34, 197, 94, 0.28);
  }

  .badge.offline {
    background: rgba(239, 68, 68, 0.12);
    color: rgba(239, 68, 68, 0.95);
    border-color: rgba(239, 68, 68, 0.28);
  }

  .actions {
    display: inline-flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .icon-btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 12px;
    padding: 8px;
    cursor: pointer;
    display: grid;
    place-items: center;
  }

  .icon-btn:hover {
    background: var(--bg-hover);
  }

  .icon-btn.danger {
    color: rgba(239, 68, 68, 0.95);
    border-color: rgba(239, 68, 68, 0.28);
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

  @media (max-width: 900px) {
    .page-content {
      padding: 18px;
    }

    .stats {
      grid-template-columns: 1fr;
    }

    .search {
      min-width: 0;
      width: 100%;
    }
  }
</style>
