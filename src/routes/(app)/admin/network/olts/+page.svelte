<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import type { Olt } from '$lib/api/olt';
  import { toast } from '$lib/stores/toast';
  import { appendBackParam } from '$lib/utils/backNavigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { appSettings } from '$lib/stores/settings';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';

  const OLT_TYPE_MAP: Record<string, string> = {
    hioso_ha7302cst: 'HIOSO HA-7302CST (EPON)',
    mikrotik_ros: 'MikroTik RouterOS (API)',
    vsol_epon: 'VSOL (EPON)',
  };

  function friendlyOltType(t: string): string {
    return OLT_TYPE_MAP[t] || t;
  }

  let loading = $state(true);
  let olts = $state<Olt[]>([]);
  let search = $state('');
  let isMobile = $state(false);

  let showModal = $state(false);
  let editing: Olt | null = $state(null);
  let showDeleteConfirm = $state(false);
  let deleteTarget = $state<Olt | null>(null);
  let testingId = $state<string | null>(null);
  let saving = $state(false);

  // Form fields
  let formName = $state('');
  let formDescription = $state('');
  let formOltType = $state('hioso_ha7302cst');
  let formHost = $state('');
  let formPort = $state(161);
  let formUsername = $state('');
  let formPassword = $state('');
  let formLatitude = $state<number | null>(null);
  let formLongitude = $state<number | null>(null);
  let formAddressLine = $state('');
  let formUplinkRouterId = $state<string>('');
  let formUplinkPort = $state('');

  // Router list for uplink dropdown
  let routers = $state<import('$lib/api/olt').MikrotikRouter[]>([]);

  const tenantPrefix = $derived($page.url.pathname.replace(/\/admin\/network\/olts.*$/, '') || '');

  // Map picker state
  let showMapPicker = $state(false);
  let pickerMapHost = $state<HTMLDivElement | null>(null);
  let pickerMap: any = null;
  let pickerMarker: any = null;
  let pickerLat = $state<number | null>(null);
  let pickerLng = $state<number | null>(null);
  let pickerViewMode = $state<'standard' | 'satellite'>('standard');
  let pickerMapLoading = $state(false);
  let pickerMapUnavailable = $state(false);
  let pickerMapErrorMessage = $state('');
  let maplibrePromise: Promise<any> | null = null;
  const pickerMapTilerKey = (import.meta.env.VITE_MAPTILER_KEY as string | undefined)?.trim();
  const pickerStandardMaxZoom = 19;
  const pickerSatelliteMaxZoom = pickerMapTilerKey ? 21 : 18;

  function parseCoordOrNull(v: number | null) {
    if (v == null) return null;
    return Number.isFinite(v) ? v : null;
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
    setVisibility('olt-picker-standard', !showSatellite);
    setVisibility('olt-picker-satellite', showSatellite);
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
              { id: 'olt-picker-standard', type: 'raster', source: 'standard' },
              {
                id: 'olt-picker-satellite',
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
    formLatitude = pickerLat;
    formLongitude = pickerLng;
    closeMapPicker();
  }

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return olts;
    return olts.filter((o) => {
      const hay = `${o.name} ${o.host} ${o.description || ''} ${friendlyOltType(o.olt_type)}`.toLowerCase();
      return hay.includes(q);
    });
  });

  const stats = $derived.by(() => {
    const total = olts.length;
    const online = olts.filter((o) => o.is_online).length;
    const offline = total - online;
    return { total, online, offline };
  });

  const columns = $derived.by(() => [
    { key: 'name', label: $t('common.name') || 'Name' },
    { key: 'type', label: $t('common.type') || 'Type' },
    { key: 'host', label: $t('network.olt.host') || 'Host' },
    { key: 'uplink', label: $t('network.olt.uplink') || 'Uplink' },
    { key: 'status', label: $t('common.status') || 'Status' },
    { key: 'seen', label: $t('network.olt.last_seen') || 'Last Seen' },
    { key: 'actions', label: '', align: 'right' as const, width: '200px' },
  ]);

  let refreshHandle: any = null;

  onMount(() => {
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) {
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
    // Load router list for uplink dropdown
    api.mikrotik.list().then((r: any) => {
      if (Array.isArray(r)) routers = r.filter((rt: any) => rt.enabled);
    }).catch(() => {});
    refreshHandle = setInterval(() => {
      void loadSilent();
    }, 10000);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
    pickerMarker?.remove();
    pickerMap?.remove();
  });

  async function load() {
    loading = true;
    try {
      olts = (await api.olt.list()) as any;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function loadSilent() {
    if (showModal) return;
    try {
      olts = (await api.olt.list()) as any;
    } catch {
      // silent
    }
  }

  function openCreate() {
    editing = null;
    formName = '';
    formDescription = '';
    formOltType = 'hioso_ha7302cst';
    formHost = '';
    formPort = 161;
    formUsername = '';
    formPassword = '';
    formLatitude = null;
    formLongitude = null;
    formAddressLine = '';
    formUplinkRouterId = '';
    formUplinkPort = '';
    showModal = true;
  }

  function openEdit(o: Olt) {
    editing = o;
    formName = o.name || '';
    formDescription = o.description || '';
    formOltType = o.olt_type || 'hioso_ha7302cst';
    formHost = o.host || '';
    formPort = o.port || 161;
    formUsername = o.username || '';
    formPassword = '';
    formLatitude = o.latitude ?? null;
    formLongitude = o.longitude ?? null;
    formAddressLine = o.address_line ?? '';
    formUplinkRouterId = o.uplink_router_id ?? '';
    formUplinkPort = o.uplink_port ?? '';
    showModal = true;
  }

  async function save() {
    const name = formName.trim();
    const host = formHost.trim();
    const username = formUsername.trim();
    if (!name || !host || !username) {
      toast.error($t('common.required_fields') || 'Please fill in all required fields.');
      return;
    }
    if (!editing && !formPassword.trim()) {
      toast.error($t('common.password_required') || 'Password is required.');
      return;
    }
    saving = true;
    try {
      if (editing) {
        // Only set uplink if router actually changed (avoids duplicate NetworkLinks)
        if (formUplinkRouterId !== (editing.uplink_router_id ?? '')) {
          await api.olt.setUplink(editing.id, {
            router_id: formUplinkRouterId || '',
            port: formUplinkPort.trim() || null,
          });
        }
        await api.olt.update(editing.id, {
          name,
          description: formDescription.trim() || null,
          host,
          port: formPort,
          username,
          password: formPassword.trim() ? formPassword : undefined,
          latitude: formLatitude,
          longitude: formLongitude,
          address_line: formAddressLine.trim() || null,
        });
        toast.success($t('network.olt.updated') || 'OLT updated successfully.');
      } else {
        await api.olt.create({
          name,
          description: formDescription.trim() || null,
          olt_type: formOltType,
          host,
          port: formPort,
          username,
          password: formPassword,
          latitude: formLatitude,
          longitude: formLongitude,
          address_line: formAddressLine.trim() || null,
          uplink_router_id: formUplinkRouterId || null,
          uplink_port: formUplinkPort.trim() || null,
        } as any);
        toast.success($t('network.olt.created') || 'OLT created successfully.');
      }
      showModal = false;
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      saving = false;
    }
  }

  function openOnMap(o: Olt) {
    if (o.latitude == null || o.longitude == null) {
      toast.error($t('network.olt.no_coordinates') || 'OLT does not have location coordinates yet.');
      return;
    }
    const params = new URLSearchParams({
      asset_id: o.id,
      asset_lat: String(o.latitude),
      asset_lng: String(o.longitude),
    });
    void goto(`${tenantPrefix}/admin/network/map?${params.toString()}`);
  }

  async function testConnection(o: Olt) {
    testingId = o.id;
    try {
      const result = await api.olt.test({
        id: o.id, // Backend uses stored credentials when id is provided
        host: o.host,
        port: o.port,
        username: o.username,
        password: '',
        olt_type: o.olt_type,
      });
      if (result?.success) {
        toast.success(`${$t('network.olt.connection_success') || 'Connection successful!'} ${result.info?.model || ''} ${result.info?.version ? `v${result.info.version}` : ''}`);
      } else {
        toast.error(result?.error || ($t('network.olt.connection_failed') || 'Connection failed.'));
      }
      await loadSilent();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      testingId = null;
    }
  }

  function remove(o: Olt) {
    deleteTarget = o;
    showDeleteConfirm = true;
  }

  async function handleConfirmDelete() {
    if (!deleteTarget) return;
    const o = deleteTarget;
    deleteTarget = null;
    try {
      await api.olt.delete(o.id);
      toast.success($t('network.olt.deleted') || 'OLT deleted successfully.');
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function openDetail(o: Olt) {
    goto(appendBackParam(`${$page.url.pathname}/${o.id}`, $page.url));
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader title={$t('network.olt.title') || 'OLT Monitoring'} subtitle={$t('network.olt.description') || 'Kelola perangkat OLT dan pantau status ONU.'}>
    {#snippet actions()}
      <button class="btn ghost" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      {#if $can('manage', 'router_inventory')}
        <button class="btn" type="button" onclick={openCreate}>
          <Icon name="plus" size={16} />
          {$t('network.olt.add') || 'Add OLT'}
        </button>
      {/if}
    {/snippet}
  </NetworkPageHeader>

  <div class="stats">
    <div class="stat-card">
      <div class="stat-top">
        <span class="stat-label">{$t('network.olt.total') || 'Total'}</span>
        <Icon name="list" size={14} />
      </div>
      <div class="stat-value">{stats.total}</div>
    </div>
    <div class="stat-card tone-ok">
      <div class="stat-top">
        <span class="stat-label">{$t('network.olt.online') || 'Online'}</span>
        <Icon name="check-circle" size={14} />
      </div>
      <div class="stat-value">{stats.online}</div>
    </div>
    <div class="stat-card tone-bad">
      <div class="stat-top">
        <span class="stat-label">{$t('network.olt.offline') || 'Offline'}</span>
        <Icon name="alert-circle" size={14} />
      </div>
      <div class="stat-value">{stats.offline}</div>
    </div>
  </div>

  <div class="toolbar">
    <div class="search">
      <Icon name="search" size={16} />
      <input class="search-input" bind:value={search} placeholder={$t('network.olt.search') || 'Cari OLT...'} />
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
      emptyText={$t('network.olt.empty') || 'No OLTs yet'}
      mobileView={isMobile ? 'card' : 'scroll'}
    >
      {#snippet cell({ item, key }: any)}
        {#if key === 'name'}
          <div class="name-cell">
            <div class="name-top">
              <span class="name">{item.name}</span>
              <span class="chip">{friendlyOltType(item.olt_type)}</span>
            </div>
            {#if item.description}
              <div class="muted">{item.description}</div>
            {/if}
            <div class="muted">{item.username}@{item.host}:{item.port}</div>
            {#if item.last_error}
              <div class="error">{item.last_error}</div>
            {/if}
          </div>
        {:else if key === 'type'}
          <span class="chip">{friendlyOltType(item.olt_type)}</span>
        {:else if key === 'host'}
          <span class="mono">{item.host}:{item.port}</span>
        {:else if key === 'uplink'}
          {#if item.uplink_router_name}
            <span>{item.uplink_router_name}</span>
          {:else if item.uplink_router_id}
            <span class="mono muted">{item.uplink_router_id}</span>
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if key === 'status'}
          <span class="badge" class:online={item.is_online} class:offline={!item.is_online}>
            {item.is_online ? ($t('network.olt.online') || 'Online') : ($t('network.olt.offline') || 'Offline')}
          </span>
        {:else if key === 'seen'}
          {#if item.last_polled_at}
            <span class="muted" title={formatDateTime(item.last_polled_at, { timeZone: $appSettings.app_timezone })}>
              {timeAgo(item.last_polled_at)}
            </span>
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if key === 'actions'}
          <div class="actions">
            <button class="icon-btn" type="button" onclick={() => openDetail(item)} title={$t('common.open') || 'Buka'}>
              <Icon name="arrow-right" size={16} />
            </button>
            <button class="icon-btn" type="button" onclick={() => openOnMap(item)} disabled={item.latitude == null || item.longitude == null} title={$t('network.olt.view_on_map') || 'Lihat di Peta'}>
              <Icon name="map-pin" size={16} />
            </button>
            <button class="icon-btn" type="button" onclick={() => testConnection(item)} disabled={testingId === item.id} title={$t('network.olt.test_connection') || 'Test Koneksi'}>
              <Icon name="zap" size={16} />
            </button>
            {#if $can('manage', 'router_inventory')}
              <button class="icon-btn" type="button" onclick={() => openEdit(item)} title={$t('common.edit') || 'Edit'}>
                <Icon name="edit" size={16} />
              </button>
              <button class="icon-btn danger" type="button" onclick={() => remove(item)} title={$t('common.delete') || 'Hapus'}>
                <Icon name="trash-2" size={16} />
              </button>
            {/if}
          </div>
        {/if}
      {/snippet}
    </Table>
  </div>
</div>

{#if showModal}
  <div class="modal-backdrop" onclick={() => (showModal = false)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>{editing ? ($t('common.edit') || 'Edit') : ($t('network.olt.add') || 'Add OLT')}</h3>
        <button class="icon-btn" type="button" onclick={() => (showModal = false)}>
          <Icon name="x" size={18} />
        </button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label for="olt-name">{$t('common.name') || 'Name'} <span class="req">*</span></label>
          <input id="olt-name" type="text" bind:value={formName} placeholder={$t('network.olt.name_placeholder') || 'e.g. Building A OLT'} />
        </div>
        <div class="form-group">
          <label for="olt-desc">{$t('common.description') || 'Description'}</label>
          <input id="olt-desc" type="text" bind:value={formDescription} placeholder={$t('network.olt.desc_placeholder') || 'OLT for Building A area'} />
        </div>
        <div class="form-group">
          <label for="olt-type">{$t('network.olt.type') || 'OLT Type'} <span class="req">*</span></label>
          <select id="olt-type" bind:value={formOltType} disabled={!!editing}>
            <option value="hioso_ha7302cst">HIOSO HA-7302CST (EPON)</option>
            <option value="vsol_epon">VSOL (EPON)</option>
          </select>
        </div>
        <div class="form-row">
          <div class="form-group flex-2">
            <label for="olt-host">{$t('common.host') || 'Host'} <span class="req">*</span></label>
            <input id="olt-host" type="text" bind:value={formHost} placeholder="192.168.1.1" />
          </div>
          <div class="form-group flex-1">
            <label for="olt-port">{$t('common.port') || 'Port'} <span class="req">*</span></label>
            <input id="olt-port" type="number" bind:value={formPort} min="1" max="65535" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group flex-1">
            <label for="olt-user">{$t('common.username') || 'Username'} <span class="req">*</span></label>
            <input id="olt-user" type="text" bind:value={formUsername} placeholder="admin" />
          </div>
          <div class="form-group flex-1">
            <label for="olt-pass">{$t('common.password') || 'Password'} {@html editing ? '' : '<span class="req">*</span>'}</label>
            <input id="olt-pass" type="password" bind:value={formPassword} placeholder={editing ? ($t('network.olt.leave_blank_to_keep') || 'Leave blank to keep current') : '••••••'} />
          </div>
          </div>
          <hr class="form-divider">
          <div class="section-label">{$t('network.olt.location') || 'OLT Location'}</div>
          <div class="form-group">
            <button class="btn ghost map-pick-btn btn-block" type="button" onclick={openMapPicker} disabled={saving}>
              <Icon name="map-pin" size={15} />
              {formLatitude != null && formLongitude != null ? `${$t('network.olt.change_point') || 'Change Point'} (${formLatitude.toFixed(6)}, ${formLongitude.toFixed(6)})` : ($t('network.olt.pick_on_map') || 'Pick Location on Map')}
            </button>
          </div>
          <div class="form-group addr-group">
          <label for="olt-addr">{$t('network.olt.address') || 'Address'}</label>
          <input id="olt-addr" type="text" bind:value={formAddressLine} placeholder={$t('network.olt.address_placeholder') || '123 Main St, District...'} />
          </div>
          <hr class="form-divider">
          <div class="section-label">{$t('network.olt.uplink_router') || 'Uplink Router'}</div>
          <div class="form-row">
            <div class="form-group flex-2">
              <label for="olt-uplink-router">{$t('network.olt.uplink_router') || 'Uplink Router'}</label>
              <select id="olt-uplink-router" bind:value={formUplinkRouterId}>
                <option value="">— {$t('common.none') || 'None'} —</option>
                {#each routers as r}
                  <option value={r.id}>{r.name} ({r.host})</option>
                {/each}
              </select>
            </div>
            <div class="form-group flex-1">
              <label for="olt-uplink-port">{$t('common.port') || 'Port'}</label>
              <input id="olt-uplink-port" type="text" bind:value={formUplinkPort} placeholder="ether1, sfp1..." />
            </div>
          </div>
          </div>
          <div class="modal-footer">
        <button class="btn ghost" type="button" onclick={() => (showModal = false)}>{$t('common.cancel') || 'Cancel'}</button>
        <button class="btn" type="button" onclick={save} disabled={saving}>
          {#if saving}
            <Icon name="loader" size={16} />
            {$t('common.saving') || 'Saving...'}
          {:else}
            {editing ? ($t('common.update') || 'Update') : ($t('common.save') || 'Save')}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:show={showDeleteConfirm}
  title={$t('common.confirm_delete_title') || 'Confirm Delete'}
  message={$t('network.olt.confirm_delete') || 'Are you sure you want to delete this OLT? This action cannot be undone.'}
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  type="danger"
  onconfirm={handleConfirmDelete}
  oncancel={() => { deleteTarget = null; }}
/>

<!-- Map Picker Modal -->
{#if showMapPicker}
  <div class="modal-backdrop" onclick={closeMapPicker} role="presentation">
    <div class="modal map-picker-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>{$t('network.olt.pick_location') || 'Pick OLT Location'}</h3>
        <button class="icon-btn" type="button" onclick={closeMapPicker}>
          <Icon name="x" size={18} />
        </button>
      </div>
      <div class="modal-body map-picker-body">
        <div class="map-picker-help">{$t('network.router.click_map_to_select') || 'Click map to select a point, then drag the marker for precision.'}</div>
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
          mapUnavailableTitle={$t('network.map.map_unavailable_title') || 'Map unavailable'}
          mapUnavailableSubtitle={$t('network.map.map_unavailable_subtitle') || 'Unable to initialize WebGL map on this browser/device.'}
          height="min(55vh, 480px)"
        />
        <div class="picker-actions">
          <button class="btn ghost" type="button" onclick={closeMapPicker}>{$t('network.map.cancel') || ($t('common.cancel') || 'Cancel')}</button>
          <button class="btn" type="button" onclick={applyPickedCoordinates}>
            <Icon name="check" size={16} />
            {$t('network.olt.use_this_point') || 'Use This Point'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .page-content {
    padding: 24px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 9px 13px;
    border-radius: 10px;
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

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 12px;
  }

  .stat-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 12px 13px 11px;
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
    margin-top: 8px;
    font-size: 1.42rem;
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
    margin-bottom: 10px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 9px 11px;
    min-width: min(500px, 100%);
    color: var(--text-secondary);
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.9rem;
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
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
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
    font-size: 0.7rem;
    font-weight: 800;
    padding: 2px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-hover), transparent 20%);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
    color: var(--text-primary);
  }

  .muted {
    color: var(--text-secondary);
    font-size: 0.86rem;
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
    padding: 5px 9px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.74rem;
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
    border-radius: 10px;
    padding: 7px;
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

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: grid;
    place-items: center;
    z-index: 1000;
    padding: 24px;
  }

  .modal {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    width: 100%;
    max-width: 520px;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .form-group label {
    font-size: 0.82rem;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .form-group input,
  .form-group select {
    padding: 9px 11px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.9rem;
    outline: none;
  }

  .form-group input:focus,
  .form-group select:focus {
    border-color: var(--color-primary);
  }
  .form-divider {
    border: none;
    border-top: 1px solid var(--border-color);
    margin: 4px 0;
  }
  .section-label {
    font-size: 0.82rem;
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .map-pick-btn {
    flex-shrink: 0;
    height: 40px;
    margin-bottom: 0;
    align-self: flex-end;
  }
  .addr-group {
    margin-top: 2px;
  }
  .map-picker-modal {
    max-width: 820px !important;
    width: 90vw !important;
  }
  .map-picker-body {
    padding: 0 !important;
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .map-picker-help {
    padding: 12px 16px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-color);
  }
  .map-picker-cords {
    padding: 8px 16px;
    font-size: 0.85rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
    background: var(--bg-hover);
    border-bottom: 1px solid var(--border-color);
  }
  .picker-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 16px;
    border-top: 1px solid var(--border-color);
  }

  .form-row {
    display: flex;
    gap: 12px;
  }

  .flex-1 {
    flex: 1;
  }

  .flex-2 {
    flex: 2;
  }

  .req {
    color: #ef4444;
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
    .form-row {
      flex-direction: column;
    }
  }

  @media (max-width: 640px) {
    .stats {
      grid-template-columns: 1fr;
    }
  }
</style>
