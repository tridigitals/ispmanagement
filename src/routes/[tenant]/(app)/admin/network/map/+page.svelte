<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onDestroy, onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { can, tenant, user } from '$lib/stores/auth';
  import { api, type PaginatedResponse } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type NMNode = {
    id: string;
    name: string;
    node_type: string;
    status: string;
    lat: number;
    lng: number;
  };

  type NMLink = {
    id: string;
    name: string;
    link_type: string;
    status: string;
    from_node_id?: string;
    to_node_id?: string;
    priority?: number;
    capacity_mbps?: number | null;
    utilization_pct?: number | null;
    loss_db?: number | null;
    latency_ms?: number | null;
    geometry: GeoJSON.Geometry;
  };

  type NMZone = {
    id: string;
    name: string;
    zone_type: string;
    status: string;
    geometry: GeoJSON.Geometry;
  };

  type MaplibreModule = typeof import('maplibre-gl');
  type MapInstance = import('maplibre-gl').Map;

  let mapEl = $state<HTMLDivElement | null>(null);
  let map = $state<MapInstance | null>(null);
  let maplibre = $state<MaplibreModule | null>(null);
  let mapReady = $state(false);
  let mapUnavailable = $state(false);
  let mapErrorMessage = $state('');
  let loading = $state(true);
  let refreshing = $state(false);
  let locating = $state(false);
  let myLocationVisible = $state(false);
  let myLocationError = $state('');

  let nodesVisible = $state(true);
  let linksVisible = $state(true);
  let zonesVisible = $state(true);
  let customersVisible = $state(true);
  let viewMode = $state<'standard' | 'satellite'>('standard');

  let q = $state('');
  let status = $state('');
  let kind = $state('');

  let nodeCount = $state(0);
  let linkCount = $state(0);
  let zoneCount = $state(0);
  let nodeRows = $state<NMNode[]>([]);
  let linkRows = $state<NMLink[]>([]);
  let zoneRows = $state<NMZone[]>([]);
  let zoneBindings = $state<any[]>([]);
  let selectedZoneId = $state('');
  let selectedTab = $state<'nodes' | 'links' | 'zones' | 'bindings'>('nodes');
  let lastLoadedZoneId = '';

  let loadingManager = $state(false);
  let savingNode = $state(false);
  let savingLink = $state(false);
  let savingZone = $state(false);
  let savingBinding = $state(false);
  let deletingId = $state<string | null>(null);
  let showDeleteConfirm = $state(false);
  let deleteTargetType = $state<'node' | 'link' | 'zone' | 'binding' | null>(null);
  let deleteTargetId = $state('');
  let deleteConfirmTitle = $state('Delete');
  let deleteConfirmMessage = $state('Are you sure?');

  let showCreateNodePanel = $state(false);
  let editingNodeId = $state<string | null>(null);
  let nodePickMode = $state(false);
  let draftNodeMarker: import('maplibre-gl').Marker | null = null;
  let nodeForm = $state({
    name: '',
    node_type: 'router',
    status: 'active',
    lat: '',
    lng: '',
  });

  let showLinkModal = $state(false);
  let editingLinkId = $state<string | null>(null);
  let linkPickMode = $state(false);
  let linkPickStep = $state<'from' | 'to'>('from');
  let linkPickDrawMode = $state<'quick' | 'path'>('quick');
  let linkPathBendPoints = $state<Array<[number, number]>>([]);
  let linkForm = $state({
    name: '',
    link_type: 'fiber',
    status: 'up',
    from_node_id: '',
    to_node_id: '',
    priority: '100',
    capacity_mbps: '',
    utilization_pct: '',
    loss_db: '',
    latency_ms: '',
    geometryText: '',
  });

  let showZoneModal = $state(false);
  let editingZoneId = $state<string | null>(null);
  let zoneForm = $state({
    name: '',
    zone_type: 'coverage',
    status: 'active',
    priority: '100',
    geometryText: '',
  });

  let bindingForm = $state({
    zone_id: '',
    node_id: '',
    is_primary: false,
    weight: '100',
  });

  let refreshDebounce: ReturnType<typeof setTimeout> | null = null;
  let lastRequestId = 0;
  let myLocationMarker: import('maplibre-gl').Marker | null = null;
  let myLocationControlBtn: HTMLButtonElement | null = null;
  let activeNodePopup: import('maplibre-gl').Popup | null = null;
  let activeDataAbortController: AbortController | null = null;
  const dataCache = new Map<
    string,
    { at: number; nodes: PaginatedResponse<any>; links: PaginatedResponse<any>; zones: PaginatedResponse<any> }
  >();
  const dataCacheTtlMs = 20_000;
  const dataCacheMaxEntries = 40;
  const mapTilerKey = (import.meta.env.VITE_MAPTILER_KEY as string | undefined)?.trim();
  const hasHiResSatellite = Boolean(mapTilerKey);
  const standardMaxZoom = 19;
  const satelliteMaxZoom = hasHiResSatellite ? 21 : 18;

  const SOURCE_NODES = 'nm-nodes';
  const SOURCE_CUSTOMERS = 'nm-customers';
  const SOURCE_LINKS = 'nm-links';
  const SOURCE_ZONES = 'nm-zones';
  const SOURCE_LINK_DRAFT = 'nm-link-draft';
  const SOURCE_LINK_DRAFT_POINTS = 'nm-link-draft-points';
  const nodeTypeOptions = [
    { label: 'Core', value: 'core' },
    { label: 'POP', value: 'pop' },
    { label: 'OLT', value: 'olt' },
    { label: 'Router', value: 'router' },
    { label: 'Tower', value: 'tower' },
    { label: 'AP', value: 'ap' },
    { label: 'Splitter', value: 'splitter' },
    { label: 'Customer Endpoint', value: 'customer_endpoint' },
  ];
  const linkTypeOptions = [
    { label: 'Fiber', value: 'fiber' },
    { label: 'Wireless PTP', value: 'wireless_ptp' },
    { label: 'Wireless PTMP', value: 'wireless_ptmp' },
    { label: 'LAN', value: 'lan' },
    { label: 'VLAN Tunnel', value: 'vlan_tunnel' },
    { label: 'Backhaul', value: 'backhaul' },
  ];
  const linkStatusOptions = [
    { label: 'Planning', value: 'planning' },
    { label: 'Up', value: 'up' },
    { label: 'Down', value: 'down' },
    { label: 'Degraded', value: 'degraded' },
    { label: 'Maintenance', value: 'maintenance' },
    { label: 'Retired', value: 'retired' },
  ];
  type LinkFieldConfig = {
    capacityLabel: string;
    utilizationLabel: string;
    latencyLabel: string;
    lossLabel: string;
    showLoss: boolean;
    helper: string;
  };

  function getLinkFieldConfig(linkType: string): LinkFieldConfig {
    switch (linkType) {
      case 'fiber':
        return {
          capacityLabel: 'Capacity (Mbps)',
          utilizationLabel: 'Utilization (%)',
          latencyLabel: 'Latency (ms)',
          lossLabel: 'Optical Loss (dB)',
          showLoss: true,
          helper: 'Fiber links track optical loss and latency for quality monitoring.',
        };
      case 'wireless_ptp':
      case 'wireless_ptmp':
        return {
          capacityLabel: 'Throughput Capacity (Mbps)',
          utilizationLabel: 'Channel Utilization (%)',
          latencyLabel: 'Latency (ms)',
          lossLabel: 'Signal Loss (dB)',
          showLoss: true,
          helper: 'Wireless links track channel utilization, latency, and signal loss.',
        };
      case 'lan':
        return {
          capacityLabel: 'Port Capacity (Mbps)',
          utilizationLabel: 'Port Utilization (%)',
          latencyLabel: 'Latency (ms)',
          lossLabel: 'Loss (dB)',
          showLoss: false,
          helper: 'LAN links focus on port capacity and utilization.',
        };
      case 'vlan_tunnel':
        return {
          capacityLabel: 'Tunnel Capacity (Mbps)',
          utilizationLabel: 'Tunnel Utilization (%)',
          latencyLabel: 'Tunnel Latency (ms)',
          lossLabel: 'Loss (dB)',
          showLoss: false,
          helper: 'VLAN tunnels focus on tunnel throughput and latency.',
        };
      case 'backhaul':
        return {
          capacityLabel: 'Backhaul Capacity (Mbps)',
          utilizationLabel: 'Backhaul Utilization (%)',
          latencyLabel: 'Backhaul Latency (ms)',
          lossLabel: 'Backhaul Loss (dB)',
          showLoss: true,
          helper: 'Backhaul links should track end-to-end latency and link loss.',
        };
      default:
        return {
          capacityLabel: 'Capacity (Mbps)',
          utilizationLabel: 'Utilization (%)',
          latencyLabel: 'Latency (ms)',
          lossLabel: 'Loss (dB)',
          showLoss: true,
          helper: 'Link quality metrics adapt based on selected type.',
        };
    }
  }

  const linkFieldConfig = $derived.by(() => getLinkFieldConfig(linkForm.link_type));

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);

  onMount(() => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }
    ensureMaplibreCompatHelpers();
    void initMap();
  });

  onDestroy(() => {
    if (refreshDebounce) clearTimeout(refreshDebounce);
    activeDataAbortController?.abort();
    myLocationMarker?.remove();
    draftNodeMarker?.remove();
    map?.remove();
  });

  $effect(() => {
    syncLayerVisibility();
  });

  $effect(() => {
    syncBaseLayerVisibility();
  });

  $effect(() => {
    syncMyLocationControlUi();
  });

  function ensureMaplibreCompatHelpers() {
    const g = globalThis as any;
    if (typeof g.__publicField !== 'function') {
      g.__publicField = (obj: any, key: PropertyKey, value: any) => {
        Object.defineProperty(obj, key, {
          value,
          enumerable: true,
          configurable: true,
          writable: true,
        });
        return value;
      };
    }
  }

  function escapeHtml(input: unknown): string {
    return String(input ?? '-')
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#039;');
  }

  function statusTone(statusRaw: unknown): 'ok' | 'warn' | 'muted' {
    const s = String(statusRaw || '').toLowerCase();
    if (s === 'active' || s === 'up') return 'ok';
    if (s === 'maintenance' || s === 'degraded') return 'warn';
    return 'muted';
  }

  function drawNodePictogram(
    ctx: CanvasRenderingContext2D,
    type: string,
    cx: number,
    cy: number,
    size: number,
  ) {
    const s = size;
    ctx.save();
    ctx.strokeStyle = '#ffffff';
    ctx.fillStyle = '#ffffff';
    ctx.lineWidth = Math.max(2, s * 0.11);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    switch (type) {
      case 'router': {
        ctx.strokeRect(cx - s * 0.48, cy - s * 0.26, s * 0.96, s * 0.52);
        ctx.beginPath();
        ctx.arc(cx - s * 0.2, cy, s * 0.05, 0, Math.PI * 2);
        ctx.arc(cx, cy, s * 0.05, 0, Math.PI * 2);
        ctx.arc(cx + s * 0.2, cy, s * 0.05, 0, Math.PI * 2);
        ctx.fill();
        break;
      }
      case 'tower': {
        ctx.beginPath();
        ctx.moveTo(cx, cy - s * 0.52);
        ctx.lineTo(cx - s * 0.28, cy + s * 0.42);
        ctx.lineTo(cx + s * 0.28, cy + s * 0.42);
        ctx.closePath();
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(cx - s * 0.18, cy + s * 0.12);
        ctx.lineTo(cx + s * 0.18, cy + s * 0.12);
        ctx.moveTo(cx - s * 0.11, cy - s * 0.12);
        ctx.lineTo(cx + s * 0.11, cy - s * 0.12);
        ctx.stroke();
        break;
      }
      case 'ap': {
        ctx.beginPath();
        ctx.arc(cx, cy + s * 0.22, s * 0.05, 0, Math.PI * 2);
        ctx.fill();
        ctx.beginPath();
        ctx.arc(cx, cy + s * 0.22, s * 0.22, -Math.PI * 0.95, -Math.PI * 0.05);
        ctx.stroke();
        ctx.beginPath();
        ctx.arc(cx, cy + s * 0.22, s * 0.36, -Math.PI * 0.9, -Math.PI * 0.1);
        ctx.stroke();
        break;
      }
      case 'olt': {
        ctx.strokeRect(cx - s * 0.5, cy - s * 0.34, s, s * 0.68);
        for (let i = -1; i <= 1; i++) {
          ctx.beginPath();
          ctx.arc(cx + i * s * 0.2, cy - s * 0.08, s * 0.045, 0, Math.PI * 2);
          ctx.fill();
          ctx.beginPath();
          ctx.moveTo(cx + i * s * 0.22, cy + s * 0.12);
          ctx.lineTo(cx + i * s * 0.22, cy + s * 0.24);
          ctx.stroke();
        }
        break;
      }
      case 'splitter': {
        ctx.beginPath();
        ctx.moveTo(cx - s * 0.45, cy);
        ctx.lineTo(cx - s * 0.05, cy);
        ctx.moveTo(cx - s * 0.05, cy);
        ctx.lineTo(cx + s * 0.3, cy - s * 0.22);
        ctx.moveTo(cx - s * 0.05, cy);
        ctx.lineTo(cx + s * 0.3, cy + s * 0.22);
        ctx.stroke();
        ctx.beginPath();
        ctx.arc(cx - s * 0.05, cy, s * 0.06, 0, Math.PI * 2);
        ctx.fill();
        break;
      }
      case 'pop': {
        ctx.strokeRect(cx - s * 0.4, cy - s * 0.4, s * 0.8, s * 0.8);
        ctx.beginPath();
        for (let r = -1; r <= 1; r++) {
          for (let c = -1; c <= 1; c++) {
            ctx.rect(cx + c * s * 0.18 - s * 0.035, cy + r * s * 0.18 - s * 0.035, s * 0.07, s * 0.07);
          }
        }
        ctx.fill();
        break;
      }
      case 'core': {
        ctx.beginPath();
        ctx.arc(cx, cy, s * 0.14, 0, Math.PI * 2);
        ctx.fill();
        ctx.beginPath();
        ctx.moveTo(cx - s * 0.42, cy);
        ctx.lineTo(cx - s * 0.18, cy);
        ctx.moveTo(cx + s * 0.18, cy);
        ctx.lineTo(cx + s * 0.42, cy);
        ctx.moveTo(cx, cy - s * 0.42);
        ctx.lineTo(cx, cy - s * 0.18);
        ctx.moveTo(cx, cy + s * 0.18);
        ctx.lineTo(cx, cy + s * 0.42);
        ctx.stroke();
        break;
      }
      case 'customer_endpoint': {
        ctx.beginPath();
        ctx.moveTo(cx - s * 0.42, cy + s * 0.08);
        ctx.lineTo(cx, cy - s * 0.34);
        ctx.lineTo(cx + s * 0.42, cy + s * 0.08);
        ctx.stroke();
        ctx.strokeRect(cx - s * 0.3, cy + s * 0.08, s * 0.6, s * 0.34);
        break;
      }
      default: {
        ctx.strokeRect(cx - s * 0.48, cy - s * 0.26, s * 0.96, s * 0.52);
        break;
      }
    }

    ctx.restore();
  }

  function buildNodeIconImage(bg: string, type: string): ImageData {
    const size = 64;
    const c = document.createElement('canvas');
    c.width = size;
    c.height = size;
    const ctx = c.getContext('2d');
    if (!ctx) return new ImageData(size, size);

    const r = size / 2;
    ctx.clearRect(0, 0, size, size);
    ctx.beginPath();
    ctx.arc(r, r + 1.2, r - 2, 0, Math.PI * 2);
    ctx.fillStyle = 'rgba(15, 23, 42, 0.35)';
    ctx.fill();

    ctx.beginPath();
    ctx.arc(r, r, r - 3, 0, Math.PI * 2);
    ctx.fillStyle = bg;
    ctx.fill();
    ctx.lineWidth = 2.6;
    ctx.strokeStyle = 'rgba(255,255,255,0.92)';
    ctx.stroke();

    drawNodePictogram(ctx, type, r, r, size * 0.56);
    return ctx.getImageData(0, 0, size, size);
  }

  function ensureNodeTypeIconsRegistered() {
    if (!map) return;
    const defs: Array<{ id: string; bg: string; type: string }> = [
      { id: 'nm-node-icon-core', bg: '#4f46e5', type: 'core' },
      { id: 'nm-node-icon-pop', bg: '#0ea5e9', type: 'pop' },
      { id: 'nm-node-icon-olt', bg: '#22c55e', type: 'olt' },
      { id: 'nm-node-icon-router', bg: '#3b82f6', type: 'router' },
      { id: 'nm-node-icon-tower', bg: '#f59e0b', type: 'tower' },
      { id: 'nm-node-icon-ap', bg: '#ef4444', type: 'ap' },
      { id: 'nm-node-icon-splitter', bg: '#a855f7', type: 'splitter' },
      { id: 'nm-node-icon-customer', bg: '#06b6d4', type: 'customer_endpoint' },
    ];
    for (const d of defs) {
      if (!map.hasImage(d.id)) {
        map.addImage(d.id, buildNodeIconImage(d.bg, d.type), { pixelRatio: 2 });
      }
    }
  }

  function handleNodeLayerClick(e: any) {
    if (!map || !e.features?.[0] || !maplibre) return;
    const props = e.features[0].properties || {};
    const coords = (e.features[0].geometry as GeoJSON.Point).coordinates;
    const nodeId = String(props.id || '');
    if (linkPickMode) {
      handleLinkPickNode(nodeId);
      return;
    }
    const popupUid = `nm-popup-${Math.random().toString(36).slice(2, 10)}`;
    const connectBtnId = `${popupUid}-connect`;
    const editBtnId = `${popupUid}-edit`;
    const closeBtnId = `${popupUid}-close`;
    const status = escapeHtml(props.status || '-');
    const tone = statusTone(props.status);
    const name = escapeHtml(props.name || '-');
    const nodeType = escapeHtml(props.node_type || '-');
    activeNodePopup?.remove();
    const popup = new maplibre.Popup({ closeButton: false, closeOnClick: true })
      .setLngLat(coords as [number, number])
      .setHTML(`
        <div class="nm-popup-card">
          <div class="nm-popup-head">
            <div class="nm-popup-title">${name}</div>
            <span class="nm-popup-badge ${tone}">${status}</span>
          </div>
          <div class="nm-popup-grid">
            <div class="nm-popup-label">Type</div>
            <div class="nm-popup-value">${nodeType}</div>
          </div>
          <div class="nm-popup-actions">
            <button id="${connectBtnId}" class="nm-popup-btn primary" type="button">Connect</button>
            <button id="${editBtnId}" class="nm-popup-btn" type="button">Edit</button>
            <button id="${closeBtnId}" class="nm-popup-btn" type="button">Close</button>
          </div>
        </div>
      `);

    popup.on('open', () => {
      const connectBtn = document.getElementById(connectBtnId) as HTMLButtonElement | null;
      const editBtn = document.getElementById(editBtnId) as HTMLButtonElement | null;
      const closeBtn = document.getElementById(closeBtnId) as HTMLButtonElement | null;

      connectBtn?.addEventListener('click', () => {
        popup.remove();
        startConnectFromNode(nodeId);
      });
      editBtn?.addEventListener('click', () => {
        popup.remove();
        const node = nodeRows.find((x) => x.id === nodeId);
        if (node) openEditNodeModal(node);
      });
      closeBtn?.addEventListener('click', () => {
        popup.remove();
      });
    });
    popup.on('close', () => {
      if (activeNodePopup === popup) activeNodePopup = null;
    });
    activeNodePopup = popup;
    popup.addTo(map);
  }

  function handleLinkLayerClick(e: any) {
    if (!map || !e.features?.[0] || !maplibre || linkPickMode) return;
    const props = e.features[0].properties || {};
    const linkId = String(props.id || '');
    const link = linkRows.find((x) => x.id === linkId);
    if (!link) return;

    const popupUid = `nm-link-popup-${Math.random().toString(36).slice(2, 10)}`;
    const deleteBtnId = `${popupUid}-delete`;
    const closeBtnId = `${popupUid}-close`;

    const p = e.lngLat;
    const name = escapeHtml(link.name || '-');
    const status = escapeHtml(link.status || '-');
    const type = escapeHtml(link.link_type || '-');
    const health = computeLinkHealth(link);
    const popup = new maplibre.Popup({ closeButton: false, closeOnClick: true })
      .setLngLat([p.lng, p.lat])
      .setHTML(`
        <div class="nm-popup-card">
          <div class="nm-popup-head">
            <div class="nm-popup-title">${name}</div>
            <span class="nm-popup-badge ${health.tone === 'good' ? 'ok' : health.tone === 'warn' ? 'warn' : 'muted'}">${health.score}</span>
          </div>
          <div class="nm-popup-grid">
            <div class="nm-popup-label">Type</div>
            <div class="nm-popup-value">${type}</div>
            <div class="nm-popup-label">Status</div>
            <div class="nm-popup-value">${status}</div>
            <div class="nm-popup-label">Endpoints</div>
            <div class="nm-popup-value mono">${escapeHtml(link.from_node_id || '-')} → ${escapeHtml(link.to_node_id || '-')}</div>
          </div>
          <div class="nm-popup-actions">
            <button id="${deleteBtnId}" class="nm-popup-btn danger" type="button">Delete</button>
            <button id="${closeBtnId}" class="nm-popup-btn" type="button">Close</button>
          </div>
        </div>
      `);

    popup.on('open', () => {
      const deleteBtn = document.getElementById(deleteBtnId) as HTMLButtonElement | null;
      const closeBtn = document.getElementById(closeBtnId) as HTMLButtonElement | null;
      deleteBtn?.addEventListener('click', () => {
        popup.remove();
        openDeleteConfirm('link', linkId, link.name);
      });
      closeBtn?.addEventListener('click', () => popup.remove());
    });
    popup.addTo(map);
  }

  async function initMap() {
    try {
      maplibre = await import('maplibre-gl');
      if (!mapEl || !maplibre) return;

      map = new maplibre.Map({
        container: mapEl,
        style: {
          version: 8,
          sources: {
            osm: {
              type: 'raster',
              tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
              tileSize: 256,
              maxzoom: standardMaxZoom,
              attribution: '© OpenStreetMap contributors',
            },
            satellite: {
              type: 'raster',
              tiles: hasHiResSatellite
                ? [`https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=${mapTilerKey}`]
                : ['https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'],
              tileSize: 256,
              maxzoom: satelliteMaxZoom,
              attribution: hasHiResSatellite ? '© MapTiler © OpenStreetMap contributors' : '© Esri',
            },
          },
          layers: [
            { id: 'base-standard', type: 'raster', source: 'osm' },
            { id: 'base-satellite', type: 'raster', source: 'satellite', layout: { visibility: 'none' } },
          ],
        },
        center: [106.8456, -6.2088],
        zoom: 10,
        maxZoom: standardMaxZoom,
        minZoom: 3,
      });

      map.addControl(new maplibre.NavigationControl({ showCompass: true, showZoom: true }), 'top-right');
      map.addControl(
        {
          onAdd: () => {
            const wrap = document.createElement('div');
            wrap.className = 'maplibregl-ctrl maplibregl-ctrl-group';

            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = 'maplibregl-ctrl-icon nm-location-ctrl';
            btn.onclick = () => {
              if (myLocationVisible) hideMyLocation();
              else void showMyLocation();
            };
            wrap.appendChild(btn);
            myLocationControlBtn = btn;
            syncMyLocationControlUi();
            return wrap;
          },
          onRemove: () => {
            myLocationControlBtn = null;
          },
        },
        'top-right',
      );

      map.on('load', async () => {
        if (!map) return;
        ensureNodeTypeIconsRegistered();
        map.addSource(SOURCE_ZONES, { type: 'geojson', data: emptyFeatureCollection() });
        map.addSource(SOURCE_LINKS, { type: 'geojson', data: emptyFeatureCollection() });
        map.addSource(SOURCE_NODES, { type: 'geojson', data: emptyFeatureCollection() });
        map.addSource(SOURCE_CUSTOMERS, {
          type: 'geojson',
          data: emptyFeatureCollection(),
          cluster: true,
          clusterMaxZoom: 14,
          clusterRadius: 54,
        });
        map.addSource(SOURCE_LINK_DRAFT, { type: 'geojson', data: emptyFeatureCollection() });
        map.addSource(SOURCE_LINK_DRAFT_POINTS, { type: 'geojson', data: emptyFeatureCollection() });

        map.addLayer({
          id: 'nm-zones-fill',
          type: 'fill',
          source: SOURCE_ZONES,
          paint: {
            'fill-color': '#1fb6ff',
            'fill-opacity': 0.12,
          },
        });

        map.addLayer({
          id: 'nm-zones-outline',
          type: 'line',
          source: SOURCE_ZONES,
          paint: {
            'line-color': '#1fb6ff',
            'line-width': 2,
            'line-opacity': 0.9,
          },
        });

        map.addLayer({
          id: 'nm-links-line',
          type: 'line',
          source: SOURCE_LINKS,
          filter: [
            'all',
            ['!=', ['get', 'status'], 'maintenance'],
            ['!=', ['get', 'status'], 'planning'],
          ],
          paint: {
            'line-color': [
              'match',
              ['get', 'health_tone'],
              'bad',
              '#ef4444',
              'warn',
              '#f59e0b',
              '#3f8cff',
            ],
            'line-width': 2.5,
            'line-opacity': 0.95,
          },
        });
        map.addLayer({
          id: 'nm-links-line-dashed',
          type: 'line',
          source: SOURCE_LINKS,
          filter: ['in', ['get', 'status'], ['literal', ['maintenance', 'planning']]],
          paint: {
            'line-color': [
              'match',
              ['get', 'health_tone'],
              'bad',
              '#ef4444',
              'warn',
              '#f59e0b',
              '#3f8cff',
            ],
            'line-width': 2.5,
            'line-opacity': 0.95,
            'line-dasharray': [1.4, 1.2],
          },
        });

        map.addLayer({
          id: 'nm-nodes-circle',
          type: 'circle',
          source: SOURCE_NODES,
          filter: ['!=', ['get', 'node_type'], 'customer_endpoint'],
          paint: {
            'circle-radius': [
              'interpolate',
              ['linear'],
              ['zoom'],
              8,
              6,
              11,
              8,
              14,
              10.5,
            ],
            'circle-color': [
              'match',
              ['get', 'status'],
              'active',
              '#16a34a',
              'maintenance',
              '#f59e0b',
              '#64748b',
            ],
            'circle-stroke-width': 1.6,
            'circle-stroke-color': '#e2e8f0',
          },
        });

        map.addLayer({
          id: 'nm-nodes-icons',
          type: 'symbol',
          source: SOURCE_NODES,
          filter: ['!=', ['get', 'node_type'], 'customer_endpoint'],
          layout: {
            'icon-image': [
              'match',
              ['get', 'node_type'],
              'core',
              'nm-node-icon-core',
              'pop',
              'nm-node-icon-pop',
              'olt',
              'nm-node-icon-olt',
              'router',
              'nm-node-icon-router',
              'tower',
              'nm-node-icon-tower',
              'ap',
              'nm-node-icon-ap',
              'splitter',
              'nm-node-icon-splitter',
              'customer_endpoint',
              'nm-node-icon-customer',
              'nm-node-icon-router',
            ],
            'icon-size': [
              'interpolate',
              ['linear'],
              ['zoom'],
              8,
              0.58,
              11,
              0.72,
              14,
              0.88,
            ],
            'icon-allow-overlap': true,
            'icon-ignore-placement': true,
          },
        });

        map.addLayer({
          id: 'nm-customers-cluster-circle',
          type: 'circle',
          source: SOURCE_CUSTOMERS,
          filter: ['has', 'point_count'],
          paint: {
            'circle-color': [
              'step',
              ['get', 'point_count'],
              '#06b6d4',
              20,
              '#3b82f6',
              60,
              '#6366f1',
            ],
            'circle-radius': [
              'step',
              ['get', 'point_count'],
              16,
              20,
              20,
              60,
              24,
            ],
            'circle-stroke-width': 1.6,
            'circle-stroke-color': '#e2e8f0',
          },
        });

        map.addLayer({
          id: 'nm-customers-cluster-count',
          type: 'symbol',
          source: SOURCE_CUSTOMERS,
          filter: ['has', 'point_count'],
          layout: {
            'text-field': ['to-string', ['get', 'point_count_abbreviated']],
            'text-size': 11,
            'text-allow-overlap': true,
          },
          paint: {
            'text-color': '#f8fafc',
          },
        });

        map.addLayer({
          id: 'nm-customers-point',
          type: 'symbol',
          source: SOURCE_CUSTOMERS,
          filter: ['!', ['has', 'point_count']],
          layout: {
            'icon-image': 'nm-node-icon-customer',
            'icon-size': [
              'interpolate',
              ['linear'],
              ['zoom'],
              8,
              0.64,
              11,
              0.8,
              14,
              0.94,
            ],
            'icon-allow-overlap': true,
            'icon-ignore-placement': true,
          },
        });

        map.addLayer({
          id: 'nm-link-draft-line',
          type: 'line',
          source: SOURCE_LINK_DRAFT,
          paint: {
            'line-color': '#38bdf8',
            'line-width': 3,
            'line-opacity': 0.9,
            'line-dasharray': [1.5, 1.2],
          },
        });

        map.addLayer({
          id: 'nm-link-draft-points',
          type: 'circle',
          source: SOURCE_LINK_DRAFT_POINTS,
          paint: {
            'circle-radius': 4.5,
            'circle-color': '#38bdf8',
            'circle-stroke-color': '#0b1020',
            'circle-stroke-width': 1.2,
          },
        });

        map.on('click', 'nm-nodes-circle', handleNodeLayerClick);
        map.on('click', 'nm-nodes-icons', handleNodeLayerClick);
        map.on('click', 'nm-customers-point', handleNodeLayerClick);
        map.on('click', 'nm-customers-cluster-circle', async (e) => {
          if (!map || !maplibre || !e.features?.[0]) return;
          const feature = e.features[0];
          const clusterId = feature.properties?.cluster_id;
          const src = map.getSource(SOURCE_CUSTOMERS) as import('maplibre-gl').GeoJSONSource | undefined;
          if (!src || clusterId == null) return;
          try {
            const zoom = await src.getClusterExpansionZoom(clusterId);
            if (!map) return;
            const coords = (feature.geometry as GeoJSON.Point).coordinates as [number, number];
            map.easeTo({ center: coords, zoom: Math.max(zoom, map.getZoom() + 1), duration: 280 });
          } catch (error) {
            console.error(error);
          }
        });
        map.on('click', 'nm-links-line', handleLinkLayerClick);
        map.on('click', 'nm-links-line-dashed', handleLinkLayerClick);

        map.on('click', (e) => {
          if (!map) return;
          if (linkPickMode && linkPickDrawMode === 'path' && linkForm.from_node_id) {
            const hitNode =
              map.queryRenderedFeatures(e.point, {
                layers: ['nm-nodes-circle', 'nm-nodes-icons', 'nm-customers-point'],
              }).length > 0;
            if (!hitNode) {
              linkPathBendPoints = [...linkPathBendPoints, [e.lngLat.lng, e.lngLat.lat]];
              refreshLinkGeometryDraft();
              syncLinkDraftPreview();
              return;
            }
          }
          if (!nodePickMode) return;
          // Ignore node popup click when user clicks existing node circle.
          const hitNode =
            map.queryRenderedFeatures(e.point, {
              layers: ['nm-nodes-circle', 'nm-nodes-icons', 'nm-customers-point'],
            }).length > 0;
          if (hitNode) return;
          applyPickedNodeCoordinates(e.lngLat.lng, e.lngLat.lat);
        });

        for (const layerId of [
          'nm-zones-fill',
          'nm-zones-outline',
          'nm-links-line',
          'nm-links-line-dashed',
          'nm-nodes-circle',
          'nm-nodes-icons',
          'nm-customers-cluster-circle',
          'nm-customers-cluster-count',
          'nm-customers-point',
          'nm-link-draft-line',
          'nm-link-draft-points',
        ]) {
          map.on('mouseenter', layerId, () => {
            if (map) map.getCanvas().style.cursor = 'pointer';
          });
          map.on('mouseleave', layerId, () => {
            if (map) map.getCanvas().style.cursor = '';
          });
        }

        map.on('moveend', scheduleRefresh);
        mapReady = true;
        syncLayerVisibility();
        syncLinkDraftPreview();
        await refreshMapData();
      });
    } catch (e: any) {
      console.error(e);
      mapUnavailable = true;
      mapErrorMessage = e?.message || 'Failed to initialize WebGL map.';
      await refreshMapData();
    } finally {
      loading = false;
    }
  }

  function emptyFeatureCollection(): GeoJSON.FeatureCollection {
    return { type: 'FeatureCollection', features: [] };
  }

  function currentBboxString(): string | null {
    if (!map) return '-180,-85,180,85';
    const b = map.getBounds();
    if (!b) return '-180,-85,180,85';
    // Keep bbox stable at very high zoom.
    // With coarse rounding, west/east (or south/north) can collapse and cause empty backend results.
    const minSpanLng = 0.0002;
    const minSpanLat = 0.0002;
    let west = b.getWest();
    let east = b.getEast();
    let south = b.getSouth();
    let north = b.getNorth();

    if (east - west < minSpanLng) {
      const mid = (east + west) / 2;
      west = mid - minSpanLng / 2;
      east = mid + minSpanLng / 2;
    }
    if (north - south < minSpanLat) {
      const mid = (north + south) / 2;
      south = mid - minSpanLat / 2;
      north = mid + minSpanLat / 2;
    }

    return `${west.toFixed(8)},${south.toFixed(8)},${east.toFixed(8)},${north.toFixed(8)}`;
  }

  function scheduleRefresh() {
    if (refreshDebounce) clearTimeout(refreshDebounce);
    refreshDebounce = setTimeout(() => {
      void refreshMapData();
    }, 280);
  }

  function invalidateMapDataCache() {
    dataCache.clear();
  }

  async function refreshMapData(force = false) {
    if (map && !mapReady) return;
    const requestId = ++lastRequestId;
    const bbox = currentBboxString();
    if (!bbox) return;

    refreshing = true;

    try {
      const params = {
        q: q.trim() || undefined,
        status: status || undefined,
        kind: kind || undefined,
        bbox,
        page: 1,
        per_page: 1000,
      };

      const zoomSig = map ? map.getZoom().toFixed(2) : '0.00';
      const cacheKey = JSON.stringify({
        q: params.q || '',
        status: params.status || '',
        kind: params.kind || '',
        bbox: params.bbox,
        zoom: zoomSig,
      });
      const cached = force ? undefined : dataCache.get(cacheKey);
      if (cached && Date.now() - cached.at <= dataCacheTtlMs) {
        if (requestId !== lastRequestId) return;
        nodeRows = (cached.nodes.data || []) as NMNode[];
        linkRows = (cached.links.data || []) as NMLink[];
        zoneRows = (cached.zones.data || []) as NMZone[];
        nodeCount = cached.nodes.total || cached.nodes.data?.length || 0;
        linkCount = cached.links.total || cached.links.data?.length || 0;
        zoneCount = cached.zones.total || cached.zones.data?.length || 0;
        setSourceData(SOURCE_NODES, nodesToFeatureCollection(cached.nodes.data as NMNode[]));
        setSourceData(SOURCE_CUSTOMERS, customersToFeatureCollection(cached.nodes.data as NMNode[]));
        setSourceData(SOURCE_LINKS, linksToFeatureCollection(cached.links.data as NMLink[]));
        setSourceData(SOURCE_ZONES, zonesToFeatureCollection(cached.zones.data as NMZone[]));
        return;
      }

      activeDataAbortController?.abort();
      const abortController = new AbortController();
      activeDataAbortController = abortController;

      const [nodesRes, linksRes, zonesRes] = await Promise.all([
        api.networkMapping.nodes.list(params, { signal: abortController.signal }),
        api.networkMapping.links.list(params, { signal: abortController.signal }),
        api.networkMapping.zones.list(params, { signal: abortController.signal }),
      ]);

      // Drop stale responses when user moves map quickly.
      if (requestId !== lastRequestId) return;
      if (abortController.signal.aborted) return;

      nodeRows = (nodesRes.data || []) as NMNode[];
      linkRows = (linksRes.data || []) as NMLink[];
      zoneRows = (zonesRes.data || []) as NMZone[];
      nodeCount = nodesRes.total || nodesRes.data?.length || 0;
      linkCount = linksRes.total || linksRes.data?.length || 0;
      zoneCount = zonesRes.total || zonesRes.data?.length || 0;

      dataCache.set(cacheKey, {
        at: Date.now(),
        nodes: nodesRes,
        links: linksRes,
        zones: zonesRes,
      });
      if (dataCache.size > dataCacheMaxEntries) {
        const oldestKey = dataCache.keys().next().value as string | undefined;
        if (oldestKey) dataCache.delete(oldestKey);
      }

      setSourceData(SOURCE_NODES, nodesToFeatureCollection(nodesRes.data as NMNode[]));
      setSourceData(SOURCE_CUSTOMERS, customersToFeatureCollection(nodesRes.data as NMNode[]));
      setSourceData(SOURCE_LINKS, linksToFeatureCollection(linksRes.data as NMLink[]));
      setSourceData(SOURCE_ZONES, zonesToFeatureCollection(zonesRes.data as NMZone[]));
    } catch (e: any) {
      if ((e?.message || '').includes('Request canceled')) return;
      console.error(e);
    } finally {
      if (requestId === lastRequestId) activeDataAbortController = null;
      refreshing = false;
    }
  }

  function setSourceData(sourceId: string, data: GeoJSON.FeatureCollection) {
    if (!map) return;
    if (!map.getSource(sourceId)) return;
    const source = map.getSource(sourceId) as import('maplibre-gl').GeoJSONSource | undefined;
    source?.setData(data as any);
  }

  function setLayerVisibility(layerId: string, visible: boolean) {
    if (!map || !map.getLayer(layerId)) return;
    map.setLayoutProperty(layerId, 'visibility', visible ? 'visible' : 'none');
  }

  function nodesToFeatureCollection(rows: NMNode[]): GeoJSON.FeatureCollection {
    return {
      type: 'FeatureCollection',
      features: (rows || []).map((row) => ({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: [row.lng, row.lat] },
        properties: {
          id: row.id,
          name: row.name,
          node_type: row.node_type,
          status: row.status,
        },
      })),
    };
  }

  function linksToFeatureCollection(rows: NMLink[]): GeoJSON.FeatureCollection {
    return {
      type: 'FeatureCollection',
      features: (rows || []).map((row) => ({
        type: 'Feature',
        geometry: row.geometry,
        properties: {
          id: row.id,
          name: row.name,
          link_type: row.link_type,
          status: row.status,
          health_score: computeLinkHealth(row).score,
          health_tone: computeLinkHealth(row).tone,
        },
      })),
    };
  }

  function customersToFeatureCollection(rows: NMNode[]): GeoJSON.FeatureCollection {
    return {
      type: 'FeatureCollection',
      features: (rows || [])
        .filter((row) => row.node_type === 'customer_endpoint')
        .map((row) => ({
          type: 'Feature',
          geometry: { type: 'Point', coordinates: [row.lng, row.lat] },
          properties: {
            id: row.id,
            name: row.name,
            node_type: row.node_type,
            status: row.status,
          },
        })),
    };
  }

  function computeLinkHealth(row: NMLink): { score: number; tone: 'good' | 'warn' | 'bad' } {
    const statusRaw = String(row.status || '').toLowerCase();
    if (statusRaw === 'down' || statusRaw === 'retired') return { score: 5, tone: 'bad' };
    let score = 100;

    if (statusRaw === 'maintenance') score -= 32;
    if (statusRaw === 'degraded') score -= 20;
    if (statusRaw === 'planning') score -= 10;
    if (statusRaw === 'inactive') score -= 12;

    const util = row.utilization_pct ?? null;
    const latency = row.latency_ms ?? null;
    const loss = row.loss_db ?? null;
    if (util != null) {
      if (util >= 90) score -= 40;
      else if (util >= 75) score -= 20;
      else if (util >= 60) score -= 10;
    }
    if (latency != null) {
      if (latency > 40) score -= 15;
      else if (latency > 20) score -= 8;
    }
    if (loss != null) {
      if (loss > 3) score -= 25;
      else if (loss > 1) score -= 12;
      else if (loss > 0.3) score -= 6;
    }

    score = Math.max(0, Math.min(100, score));
    const tone: 'good' | 'warn' | 'bad' = score >= 80 ? 'good' : score >= 60 ? 'warn' : 'bad';
    return { score, tone };
  }

  function zonesToFeatureCollection(rows: NMZone[]): GeoJSON.FeatureCollection {
    return {
      type: 'FeatureCollection',
      features: (rows || []).map((row) => ({
        type: 'Feature',
        geometry: row.geometry,
        properties: {
          id: row.id,
          name: row.name,
          zone_type: row.zone_type,
          status: row.status,
        },
      })),
    };
  }

  function syncLayerVisibility() {
    if (!map || !mapReady) return;
    setLayerVisibility('nm-zones-fill', zonesVisible);
    setLayerVisibility('nm-zones-outline', zonesVisible);
    setLayerVisibility('nm-links-line', linksVisible);
    setLayerVisibility('nm-links-line-dashed', linksVisible);
    setLayerVisibility('nm-nodes-circle', nodesVisible);
    setLayerVisibility('nm-nodes-icons', nodesVisible);
    setLayerVisibility('nm-customers-cluster-circle', customersVisible);
    setLayerVisibility('nm-customers-cluster-count', customersVisible);
    setLayerVisibility('nm-customers-point', customersVisible);
  }

  function syncBaseLayerVisibility() {
    if (!map || !mapReady) return;
    setLayerVisibility('base-standard', viewMode === 'standard');
    setLayerVisibility('base-satellite', viewMode === 'satellite');
    const targetMaxZoom = viewMode === 'satellite' ? satelliteMaxZoom : standardMaxZoom;
    map.setMaxZoom(targetMaxZoom);
    if (map.getZoom() > targetMaxZoom) {
      map.zoomTo(targetMaxZoom, { duration: 160 });
    }
  }

  async function onApplyFilters() {
    await refreshMapData();
  }

  function onResetFilters() {
    q = '';
    status = '';
    kind = '';
    void refreshMapData();
  }

  async function showMyLocation() {
    if (!map || !maplibre || mapUnavailable || locating) return;
    if (!navigator.geolocation) {
      myLocationError = 'Geolocation is not supported by this browser.';
      return;
    }

    locating = true;
    myLocationError = '';
    try {
      const pos = await new Promise<GeolocationPosition>((resolve, reject) => {
        navigator.geolocation.getCurrentPosition(resolve, reject, {
          enableHighAccuracy: true,
          timeout: 12000,
          maximumAge: 30000,
        });
      });

      const lng = pos.coords.longitude;
      const lat = pos.coords.latitude;

      if (!myLocationMarker) {
        const el = document.createElement('div');
        el.className = 'my-location-dot';
        myLocationMarker = new maplibre.Marker({ element: el, anchor: 'center' });
      }

      myLocationMarker.setLngLat([lng, lat]).addTo(map);
      myLocationVisible = true;
      map.flyTo({ center: [lng, lat], zoom: Math.max(map.getZoom(), 15), speed: 1.1 });
    } catch (e: any) {
      myLocationError = e?.message || 'Unable to get current location.';
      console.error(e);
    } finally {
      locating = false;
    }
  }

  function hideMyLocation() {
    myLocationMarker?.remove();
    myLocationVisible = false;
    myLocationError = '';
  }

  function syncMyLocationControlUi() {
    if (!myLocationControlBtn) return;
    const showLabel = $t('admin.network.map.location.show') || 'My Location';
    const hideLabel = $t('admin.network.map.location.hide') || 'Hide My Location';
    const label = myLocationVisible ? hideLabel : showLabel;

    myLocationControlBtn.disabled = locating || mapUnavailable;
    myLocationControlBtn.title = label;
    myLocationControlBtn.setAttribute('aria-label', label);
    myLocationControlBtn.textContent = locating ? '↻' : '◎';
    myLocationControlBtn.classList.toggle('active', myLocationVisible);
    myLocationControlBtn.classList.toggle('loading', locating);
  }

  function asNumber(input: string): number | undefined {
    const value = Number.parseFloat(input);
    return Number.isFinite(value) ? value : undefined;
  }

  function prettyGeometry(value: GeoJSON.Geometry): string {
    return JSON.stringify(value, null, 2);
  }

  function parseGeometryText(text: string): GeoJSON.Geometry {
    const parsed = JSON.parse(text || '{}');
    if (!parsed || typeof parsed !== 'object' || !parsed.type) {
      throw new Error('Geometry JSON is invalid');
    }
    return parsed as GeoJSON.Geometry;
  }

  function defaultZoneGeometry() {
    if (map) {
      const b = map.getBounds();
      const w = b.getWest();
      const s = b.getSouth();
      const e = b.getEast();
      const n = b.getNorth();
      const padLng = (e - w) * 0.2;
      const padLat = (n - s) * 0.2;
      return {
        type: 'Polygon',
        coordinates: [
          [
            [w + padLng, s + padLat],
            [w + padLng, n - padLat],
            [e - padLng, n - padLat],
            [e - padLng, s + padLat],
            [w + padLng, s + padLat],
          ],
        ],
      } as GeoJSON.Polygon;
    }
    return {
      type: 'Polygon',
      coordinates: [
        [
          [106.81, -6.24],
          [106.81, -6.17],
          [106.92, -6.17],
          [106.92, -6.24],
          [106.81, -6.24],
        ],
      ],
    } as GeoJSON.Polygon;
  }

  function buildDefaultLineGeometry(fromId: string, toId: string): GeoJSON.Geometry {
    const from = nodeRows.find((x) => x.id === fromId);
    const to = nodeRows.find((x) => x.id === toId);
    if (!from || !to) {
      return {
        type: 'LineString',
        coordinates: [
          [106.84, -6.2],
          [106.87, -6.21],
        ],
      };
    }
    return {
      type: 'LineString',
      coordinates: [
        [from.lng, from.lat],
        [to.lng, to.lat],
      ],
    };
  }

  function getNodeCoord(nodeId: string): [number, number] | null {
    const n = nodeRows.find((x) => x.id === nodeId);
    return n ? [n.lng, n.lat] : null;
  }

  function currentDraftPathCoords(includeToNode = false): Array<[number, number]> {
    const coords: Array<[number, number]> = [];
    const fromCoord = linkForm.from_node_id ? getNodeCoord(linkForm.from_node_id) : null;
    if (fromCoord) coords.push(fromCoord);
    if (linkPathBendPoints.length > 0) coords.push(...linkPathBendPoints);
    if (includeToNode && linkForm.to_node_id) {
      const toCoord = getNodeCoord(linkForm.to_node_id);
      if (toCoord) coords.push(toCoord);
    }
    return coords;
  }

  function refreshLinkGeometryDraft() {
    const coords =
      linkPickDrawMode === 'path'
        ? currentDraftPathCoords(Boolean(linkForm.to_node_id))
        : ((buildDefaultLineGeometry(linkForm.from_node_id, linkForm.to_node_id) as GeoJSON.LineString)
            .coordinates as Array<[number, number]>);
    if (coords.length >= 2) {
      linkForm.geometryText = prettyGeometry({ type: 'LineString', coordinates: coords });
    }
  }

  function syncLinkDraftPreview() {
    const lineFc: GeoJSON.FeatureCollection = emptyFeatureCollection();
    const pointsFc: GeoJSON.FeatureCollection = emptyFeatureCollection();

    if (linkPickMode) {
      const lineCoords =
        linkPickDrawMode === 'path'
          ? currentDraftPathCoords(false)
          : currentDraftPathCoords(Boolean(linkForm.to_node_id));
      if (lineCoords.length >= 2) {
        lineFc.features.push({
          type: 'Feature',
          geometry: { type: 'LineString', coordinates: lineCoords },
          properties: {},
        } as GeoJSON.Feature);
      }
      for (const p of lineCoords) {
        pointsFc.features.push({
          type: 'Feature',
          geometry: { type: 'Point', coordinates: p },
          properties: {},
        } as GeoJSON.Feature);
      }
    }

    setSourceData(SOURCE_LINK_DRAFT, lineFc);
    setSourceData(SOURCE_LINK_DRAFT_POINTS, pointsFc);
    setLayerVisibility('nm-link-draft-line', linkPickMode);
    setLayerVisibility('nm-link-draft-points', linkPickMode);
  }

  function stopNodePickMode(removeMarker = false) {
    nodePickMode = false;
    if (removeMarker) {
      draftNodeMarker?.remove();
      draftNodeMarker = null;
    }
  }

  function applyPickedNodeCoordinates(lng: number, lat: number) {
    nodeForm.lat = lat.toFixed(6);
    nodeForm.lng = lng.toFixed(6);
    if (!maplibre || !map) return;
    if (!draftNodeMarker) {
      draftNodeMarker = new maplibre.Marker({ color: '#3f8cff', draggable: true })
        .setLngLat([lng, lat])
        .addTo(map);
      draftNodeMarker.on('dragend', () => {
        if (!draftNodeMarker) return;
        const p = draftNodeMarker.getLngLat();
        nodeForm.lat = p.lat.toFixed(6);
        nodeForm.lng = p.lng.toFixed(6);
      });
      return;
    }
    draftNodeMarker.setLngLat([lng, lat]);
  }

  function openCreateNodeModal() {
    editingNodeId = null;
    nodeForm = { name: '', node_type: 'router', status: 'active', lat: '', lng: '' };
    nodePickMode = true;
    if (map) {
      const center = map.getCenter();
      applyPickedNodeCoordinates(center.lng, center.lat);
    }
    showCreateNodePanel = true;
  }

  function openEditNodeModal(row: NMNode) {
    nodePickMode = true;
    applyPickedNodeCoordinates(row.lng, row.lat);
    editingNodeId = row.id;
    nodeForm = {
      name: row.name || '',
      node_type: row.node_type || 'router',
      status: row.status || 'active',
      lat: String(row.lat ?? ''),
      lng: String(row.lng ?? ''),
    };
    showCreateNodePanel = true;
  }

  function closeNodeModal() {
    showCreateNodePanel = false;
    stopNodePickMode(true);
    editingNodeId = null;
  }

  async function submitNode() {
    const lat = asNumber(nodeForm.lat);
    const lng = asNumber(nodeForm.lng);
    if (!nodeForm.name.trim() || !nodeForm.node_type.trim() || lat === undefined || lng === undefined) {
      toast.error('Please fill name, type, latitude and longitude');
      return;
    }

    savingNode = true;
    try {
      const payload = {
        name: nodeForm.name.trim(),
        node_type: nodeForm.node_type.trim(),
        status: nodeForm.status || 'active',
        lat,
        lng,
      };
      if (editingNodeId) {
        await api.networkMapping.nodes.update(editingNodeId, payload);
      } else {
        await api.networkMapping.nodes.create(payload);
      }
      toast.success(editingNodeId ? 'Node updated' : 'Node created');
      closeNodeModal();
      invalidateMapDataCache();
      await refreshMapData(true);
      await loadZoneBindings();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save node');
    } finally {
      savingNode = false;
    }
  }

  function openCreateLinkModal() {
    editingLinkId = null;
    linkPickMode = false;
    linkPickStep = 'from';
    linkPickDrawMode = 'quick';
    linkPathBendPoints = [];
    linkForm = {
      name: '',
      link_type: 'fiber',
      status: 'up',
      from_node_id: nodeRows[0]?.id || '',
      to_node_id: nodeRows[1]?.id || nodeRows[0]?.id || '',
      priority: '100',
      capacity_mbps: '',
      utilization_pct: '',
      loss_db: '',
      latency_ms: '',
      geometryText: prettyGeometry(buildDefaultLineGeometry(nodeRows[0]?.id || '', nodeRows[1]?.id || nodeRows[0]?.id || '')),
    };
    showLinkModal = true;
    syncLinkDraftPreview();
  }

  function openEditLinkModal(row: NMLink) {
    linkPickMode = false;
    linkPickStep = 'from';
    linkPickDrawMode = 'quick';
    linkPathBendPoints = [];
    editingLinkId = row.id;
    linkForm = {
      name: row.name || '',
      link_type: row.link_type || 'fiber',
      status: row.status || 'active',
      from_node_id: row.from_node_id || '',
      to_node_id: row.to_node_id || '',
      priority: String(row.priority ?? 100),
      capacity_mbps: row.capacity_mbps == null ? '' : String(row.capacity_mbps),
      utilization_pct: row.utilization_pct == null ? '' : String(row.utilization_pct),
      loss_db: row.loss_db == null ? '' : String(row.loss_db),
      latency_ms: row.latency_ms == null ? '' : String(row.latency_ms),
      geometryText: prettyGeometry((row.geometry as GeoJSON.Geometry) || buildDefaultLineGeometry('', '')),
    };
    showLinkModal = true;
    syncLinkDraftPreview();
  }

  function toggleLinkPickMode() {
    linkPickMode = !linkPickMode;
    linkPickStep = 'from';
    if (linkPickMode) {
      linkForm.from_node_id = '';
      linkForm.to_node_id = '';
      linkPathBendPoints = [];
      if (linkPickDrawMode === 'quick') {
        linkForm.geometryText = prettyGeometry(buildDefaultLineGeometry('', ''));
        toast.info('Quick mode: click source node, then destination node.');
      } else {
        linkForm.geometryText = prettyGeometry({
          type: 'LineString',
          coordinates: [],
        } as GeoJSON.LineString);
        toast.info('Path mode: click source node, add bend points on map, then click destination node.');
      }
    }
    syncLinkDraftPreview();
  }

  function closeLinkModal() {
    showLinkModal = false;
    linkPickMode = false;
    linkPickStep = 'from';
    linkPickDrawMode = 'quick';
    linkPathBendPoints = [];
    syncLinkDraftPreview();
  }

  function setLinkPickDrawMode(mode: 'quick' | 'path') {
    linkPickDrawMode = mode;
    linkPathBendPoints = [];
    linkForm.from_node_id = '';
    linkForm.to_node_id = '';
    linkPickStep = 'from';
    if (mode === 'quick') {
      linkForm.geometryText = prettyGeometry(buildDefaultLineGeometry('', ''));
    } else {
      linkForm.geometryText = prettyGeometry({ type: 'LineString', coordinates: [] } as GeoJSON.LineString);
    }
    if (linkPickMode) {
      toast.info(
        mode === 'quick'
          ? 'Quick mode active: click source then destination node.'
          : 'Path mode active: click source node, add bend points, then click destination node.',
      );
    }
    syncLinkDraftPreview();
  }

  function undoLinkPathPoint() {
    if (linkPathBendPoints.length === 0) return;
    linkPathBendPoints = linkPathBendPoints.slice(0, -1);
    refreshLinkGeometryDraft();
    syncLinkDraftPreview();
  }

  function clearLinkPathPoints() {
    linkPathBendPoints = [];
    refreshLinkGeometryDraft();
    syncLinkDraftPreview();
  }

  function hasExistingLinkBetweenNodes(fromNodeId: string, toNodeId: string, excludeLinkId?: string | null): boolean {
    if (!fromNodeId || !toNodeId || fromNodeId === toNodeId) return false;
    return linkRows.some((row) => {
      if (excludeLinkId && row.id === excludeLinkId) return false;
      return (
        (row.from_node_id === fromNodeId && row.to_node_id === toNodeId) ||
        (row.from_node_id === toNodeId && row.to_node_id === fromNodeId)
      );
    });
  }

  function cancelLinkPicking() {
    if (!linkPickMode) return;
    linkPickMode = false;
    linkPickStep = 'from';
    linkPathBendPoints = [];
    syncLinkDraftPreview();
    toast.info('Link drawing canceled.');
  }

  function handleLinkPickNode(nodeId: string) {
    if (!linkPickMode) return;
    if (linkPickStep === 'from') {
      linkForm.from_node_id = nodeId;
      linkForm.to_node_id = '';
      linkPathBendPoints = [];
      linkPickStep = 'to';
      if (linkPickDrawMode === 'quick') {
        toast.info('Source selected. Click destination node.');
      } else {
        refreshLinkGeometryDraft();
        toast.info('Source selected. Click map to add bend points, then click destination node.');
      }
      syncLinkDraftPreview();
      return;
    }
    if (linkForm.from_node_id === nodeId) {
      toast.error('Destination node must be different.');
      return;
    }
    if (hasExistingLinkBetweenNodes(linkForm.from_node_id, nodeId, editingLinkId)) {
      toast.error('This node pair already has a link. Choose another destination node.');
      return;
    }
    linkForm.to_node_id = nodeId;
    if (linkPickDrawMode === 'quick') {
      useLinkFromNodePoints();
    } else {
      refreshLinkGeometryDraft();
    }
    linkPickMode = false;
    linkPickStep = 'from';
    if (!linkForm.name.trim()) {
      const fromName = nodeRows.find((x) => x.id === linkForm.from_node_id)?.name || 'Source';
      const toName = nodeRows.find((x) => x.id === linkForm.to_node_id)?.name || 'Destination';
      linkForm.name = `Link ${fromName} -> ${toName}`;
    }
    showLinkModal = true;
    syncLinkDraftPreview();
    toast.success('Endpoints selected from map.');
  }

  function startConnectFromNode(nodeId: string) {
    activeNodePopup?.remove();
    const sourceNode = nodeRows.find((x) => x.id === nodeId);
    editingLinkId = null;
    showLinkModal = false;
    linkPickDrawMode = 'path';
    linkPickMode = true;
    linkPickStep = 'to';
    linkPathBendPoints = [];
    linkForm = {
      name: sourceNode ? `Link ${sourceNode.name}` : '',
      link_type: 'fiber',
      status: 'up',
      from_node_id: nodeId,
      to_node_id: '',
      priority: '100',
      capacity_mbps: '',
      utilization_pct: '',
      loss_db: '',
      latency_ms: '',
      geometryText: prettyGeometry({ type: 'LineString', coordinates: [] } as GeoJSON.LineString),
    };
    refreshLinkGeometryDraft();
    syncLinkDraftPreview();
    selectedTab = 'links';
    toast.info('Connect mode active: draw path on map, then click destination marker.');
  }

  function useLinkFromNodePoints() {
    const geo = buildDefaultLineGeometry(linkForm.from_node_id, linkForm.to_node_id);
    linkForm.geometryText = prettyGeometry(geo);
    syncLinkDraftPreview();
  }

  async function submitLink() {
    if (!linkForm.name.trim() || !linkForm.from_node_id || !linkForm.to_node_id) {
      toast.error('Please fill name and endpoint nodes');
      return;
    }
    if (linkForm.from_node_id === linkForm.to_node_id) {
      toast.error('Source and destination must be different nodes.');
      return;
    }
    if (hasExistingLinkBetweenNodes(linkForm.from_node_id, linkForm.to_node_id, editingLinkId)) {
      toast.error('A link between these nodes already exists.');
      return;
    }

    let geometry: GeoJSON.Geometry;
    try {
      geometry = parseGeometryText(linkForm.geometryText);
    } catch (e: any) {
      toast.error(e?.message || 'Geometry JSON is invalid');
      return;
    }

    savingLink = true;
    try {
      const payload = {
        name: linkForm.name.trim(),
        link_type: linkForm.link_type || 'fiber',
        status: linkForm.status || 'up',
        from_node_id: linkForm.from_node_id,
        to_node_id: linkForm.to_node_id,
        priority: Number.parseInt(linkForm.priority || '100', 10),
        capacity_mbps: asNumber(linkForm.capacity_mbps),
        utilization_pct: asNumber(linkForm.utilization_pct),
        loss_db: linkFieldConfig.showLoss ? asNumber(linkForm.loss_db) : null,
        latency_ms: asNumber(linkForm.latency_ms),
        geometry,
      };
      if (editingLinkId) {
        await api.networkMapping.links.update(editingLinkId, payload);
      } else {
        await api.networkMapping.links.create(payload);
      }
      toast.success(editingLinkId ? 'Link updated' : 'Link created');
      closeLinkModal();
      invalidateMapDataCache();
      await refreshMapData(true);
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save link');
    } finally {
      savingLink = false;
    }
  }

  function openCreateZoneModal() {
    editingZoneId = null;
    zoneForm = {
      name: '',
      zone_type: 'coverage',
      status: 'active',
      priority: '100',
      geometryText: prettyGeometry(defaultZoneGeometry()),
    };
    showZoneModal = true;
  }

  function openEditZoneModal(row: NMZone) {
    editingZoneId = row.id;
    zoneForm = {
      name: row.name || '',
      zone_type: row.zone_type || 'coverage',
      status: row.status || 'active',
      priority: '100',
      geometryText: prettyGeometry((row.geometry as GeoJSON.Geometry) || defaultZoneGeometry()),
    };
    showZoneModal = true;
  }

  async function submitZone() {
    if (!zoneForm.name.trim()) {
      toast.error('Zone name is required');
      return;
    }
    let geometry: GeoJSON.Geometry;
    try {
      geometry = parseGeometryText(zoneForm.geometryText);
    } catch (e: any) {
      toast.error(e?.message || 'Geometry JSON is invalid');
      return;
    }
    savingZone = true;
    try {
      const payload = {
        name: zoneForm.name.trim(),
        zone_type: zoneForm.zone_type || 'coverage',
        status: zoneForm.status || 'active',
        priority: Number.parseInt(zoneForm.priority || '100', 10),
        geometry,
      };
      if (editingZoneId) {
        await api.networkMapping.zones.update(editingZoneId, payload);
      } else {
        await api.networkMapping.zones.create(payload);
      }
      toast.success(editingZoneId ? 'Zone updated' : 'Zone created');
      showZoneModal = false;
      invalidateMapDataCache();
      await refreshMapData(true);
      await loadZoneBindings();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save zone');
    } finally {
      savingZone = false;
    }
  }

  async function removeNode(id: string) {
    deletingId = id;
    try {
      await api.networkMapping.nodes.delete(id);
      toast.success('Node deleted');
      invalidateMapDataCache();
      await refreshMapData(true);
      await loadZoneBindings();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to delete node');
    } finally {
      deletingId = null;
    }
  }

  async function removeLink(id: string) {
    deletingId = id;
    try {
      await api.networkMapping.links.delete(id);
      toast.success('Link deleted');
      invalidateMapDataCache();
      await refreshMapData(true);
    } catch (e: any) {
      toast.error(e?.message || 'Failed to delete link');
    } finally {
      deletingId = null;
    }
  }

  async function removeZone(id: string) {
    deletingId = id;
    try {
      await api.networkMapping.zones.delete(id);
      toast.success('Zone deleted');
      if (selectedZoneId === id) selectedZoneId = '';
      invalidateMapDataCache();
      await refreshMapData(true);
      await loadZoneBindings();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to delete zone');
    } finally {
      deletingId = null;
    }
  }

  async function loadZoneBindings() {
    loadingManager = true;
    try {
      const rows = await api.networkMapping.zoneBindings.list({ zone_id: selectedZoneId || undefined });
      zoneBindings = rows || [];
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load zone bindings');
    } finally {
      loadingManager = false;
    }
  }

  async function createZoneBinding() {
    if (!bindingForm.zone_id || !bindingForm.node_id) {
      toast.error('Please choose zone and node');
      return;
    }
    savingBinding = true;
    try {
      await api.networkMapping.zoneBindings.create({
        zone_id: bindingForm.zone_id,
        node_id: bindingForm.node_id,
        is_primary: bindingForm.is_primary,
        weight: Number.parseInt(bindingForm.weight || '100', 10),
      });
      toast.success('Binding created');
      bindingForm = { zone_id: bindingForm.zone_id, node_id: '', is_primary: false, weight: '100' };
      await loadZoneBindings();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to create binding');
    } finally {
      savingBinding = false;
    }
  }

  async function removeBinding(id: string) {
    deletingId = id;
    try {
      await api.networkMapping.zoneBindings.delete(id);
      toast.success('Binding deleted');
      await loadZoneBindings();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to delete binding');
    } finally {
      deletingId = null;
    }
  }

  function openDeleteConfirm(
    targetType: 'node' | 'link' | 'zone' | 'binding',
    id: string,
    name?: string,
  ) {
    deleteTargetType = targetType;
    deleteTargetId = id;
    const label = name?.trim() ? `"${name.trim()}"` : 'this item';
    if (targetType === 'node') {
      deleteConfirmTitle = 'Delete Node';
      deleteConfirmMessage = `Delete node ${label}? This action cannot be undone.`;
    } else if (targetType === 'link') {
      deleteConfirmTitle = 'Delete Link';
      deleteConfirmMessage = `Delete link ${label}? This action cannot be undone.`;
    } else if (targetType === 'zone') {
      deleteConfirmTitle = 'Delete Zone';
      deleteConfirmMessage = `Delete zone ${label}? This action cannot be undone.`;
    } else {
      deleteConfirmTitle = 'Delete Binding';
      deleteConfirmMessage = `Delete binding ${label}? This action cannot be undone.`;
    }
    showDeleteConfirm = true;
  }

  async function confirmDeleteAction() {
    if (!deleteTargetType || !deleteTargetId) {
      showDeleteConfirm = false;
      return;
    }
    const type = deleteTargetType;
    const id = deleteTargetId;
    showDeleteConfirm = false;
    if (type === 'node') {
      await removeNode(id);
    } else if (type === 'link') {
      await removeLink(id);
    } else if (type === 'zone') {
      await removeZone(id);
    } else {
      await removeBinding(id);
    }
    deleteTargetType = null;
    deleteTargetId = '';
  }

  $effect(() => {
    if (!selectedZoneId) {
      if (bindingForm.zone_id) bindingForm = { ...bindingForm, zone_id: '' };
      zoneBindings = [];
      lastLoadedZoneId = '';
      return;
    }
    if (bindingForm.zone_id !== selectedZoneId) {
      bindingForm = { ...bindingForm, zone_id: selectedZoneId };
    }
    if (lastLoadedZoneId !== selectedZoneId) {
      lastLoadedZoneId = selectedZoneId;
      void loadZoneBindings();
    }
  });
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={$t('admin.network.map.title') || 'Network Topology Map'}
    subtitle={$t('admin.network.map.subtitle') || 'Visualize nodes, links, and service zones in current viewport.'}
  >
    {#snippet actions()}
      <a class="btn ghost" href={`${tenantPrefix}/admin/network/noc`}>
        <Icon name="arrow-left" size={16} />
        {$t('admin.network.map.back_to_noc') || 'Back to NOC'}
      </a>
      <button class="btn" type="button" onclick={() => void refreshMapData()} disabled={refreshing || loading}>
        <Icon name="refresh-cw" size={16} />
        {refreshing ? ($t('common.loading') || 'Loading...') : ($t('common.refresh') || 'Refresh')}
      </button>
    {/snippet}
  </NetworkPageHeader>

  <div class="stats">
    <div class="stat-card">
      <div class="stat-top">
        <span>{$t('admin.network.map.stats.nodes') || 'Nodes'}</span>
        <Icon name="map-pin" size={16} />
      </div>
      <div class="stat-value">{nodeCount}</div>
    </div>
    <div class="stat-card tone-ok">
      <div class="stat-top">
        <span>{$t('admin.network.map.stats.links') || 'Links'}</span>
        <Icon name="git-merge" size={16} />
      </div>
      <div class="stat-value">{linkCount}</div>
    </div>
    <div class="stat-card tone-warn">
      <div class="stat-top">
        <span>{$t('admin.network.map.stats.zones') || 'Zones'}</span>
        <Icon name="layers" size={16} />
      </div>
      <div class="stat-value">{zoneCount}</div>
    </div>
  </div>

  <div class="filters-wrap">
    <NetworkFilterPanel>
      <div class="control">
        <label for="nm-search">{$t('admin.network.map.filters.search') || 'Search'}</label>
        <input
          id="nm-search"
          class="input"
          type="text"
          bind:value={q}
          placeholder={$t('admin.network.map.filters.search_placeholder') || 'Search node/link/zone...'}
          onkeydown={(e) => e.key === 'Enter' && void onApplyFilters()}
        />
      </div>

      <div class="control">
        <label for="nm-status">{$t('admin.network.map.filters.status') || 'Status'}</label>
        <select id="nm-status" class="input" bind:value={status}>
          <option value="">{$t('admin.network.map.filters.any_status') || 'Any status'}</option>
          <option value="active">Active</option>
          <option value="inactive">Inactive</option>
          <option value="maintenance">Maintenance</option>
          <option value="up">Up</option>
          <option value="down">Down</option>
          <option value="degraded">Degraded</option>
        </select>
      </div>

      <div class="control">
        <label for="nm-kind">{$t('admin.network.map.filters.kind') || 'Type'}</label>
        <select id="nm-kind" class="input" bind:value={kind}>
          <option value="">{$t('admin.network.map.filters.any_kind') || 'Any type'}</option>
          <option value="core">Core</option>
          <option value="pop">POP</option>
          <option value="olt">OLT</option>
          <option value="router">Router</option>
          <option value="tower">Tower</option>
          <option value="ap">AP</option>
          <option value="splitter">Splitter</option>
          <option value="customer_endpoint">Customer Endpoint</option>
          <option value="fiber">Fiber</option>
          <option value="lan">LAN</option>
          <option value="wireless">Wireless</option>
          <option value="ptp_radio">PTP Radio</option>
        </select>
      </div>

      <div class="control control-actions">
        <div class="control-spacer" aria-hidden="true"></div>
        <button class="btn" type="button" onclick={() => void onApplyFilters()} disabled={refreshing || loading}>
          <Icon name="check" size={14} />
          {$t('common.apply') || 'Apply'}
        </button>
        <button class="btn ghost" type="button" onclick={onResetFilters} disabled={refreshing || loading}>
          <Icon name="x-circle" size={14} />
          {$t('common.reset') || 'Reset'}
        </button>
      </div>
    </NetworkFilterPanel>
  </div>

  <div class="toolbar-wrap">
    <div class="map-toolbar">
      <div class="layer-toggles">
        <label class="toggle">
          <input type="checkbox" bind:checked={nodesVisible} />
          <span>{$t('admin.network.map.layers.nodes') || 'Nodes'}</span>
        </label>
        <label class="toggle">
          <input type="checkbox" bind:checked={linksVisible} />
          <span>{$t('admin.network.map.layers.links') || 'Links'}</span>
        </label>
        <label class="toggle">
          <input type="checkbox" bind:checked={zonesVisible} />
          <span>{$t('admin.network.map.layers.zones') || 'Zones'}</span>
        </label>
        <label class="toggle">
          <input type="checkbox" bind:checked={customersVisible} />
          <span>Customers</span>
        </label>
      </div>
    </div>

    {#if myLocationError}
      <div class="location-error">
        <Icon name="alert-triangle" size={14} />
        <span>{myLocationError}</span>
      </div>
    {/if}
  </div>

  <div class="map-wrap">
    <div class="map-shell">
      {#if loading}
        <div class="map-loading">{$t('common.loading') || 'Loading...'}</div>
      {/if}
      {#if mapUnavailable}
        <div class="map-unavailable">
          <Icon name="alert-triangle" size={16} />
          <div>
            <div class="map-unavailable-title">Map preview unavailable on this device</div>
            <div class="map-unavailable-sub">
              WebGL context failed. Data is still loaded and counts are visible.
            </div>
            <div class="map-unavailable-sub">{mapErrorMessage}</div>
          </div>
        </div>
      {:else}
        <div class="map-canvas" bind:this={mapEl}></div>
      {/if}

      <div class="map-view-switch" role="group" aria-label="Map view mode">
        <button
          type="button"
          class={`switch-btn ${viewMode === 'standard' ? 'active' : ''}`}
          onclick={() => (viewMode = 'standard')}
          title="Standard view"
          aria-label="Standard view"
        >
          Map
        </button>
        <button
          type="button"
          class={`switch-btn ${viewMode === 'satellite' ? 'active' : ''}`}
          onclick={() => (viewMode = 'satellite')}
          title="Satellite view"
          aria-label="Satellite view"
        >
          Sat
        </button>
      </div>

      {#if showCreateNodePanel}
        <div class="node-create-panel">
          <div class="panel-head">
            <div class="panel-title">{editingNodeId ? 'Edit Node' : 'Add Node'}</div>
            {!editingNodeId
              ? (nodePickMode
                  ? 'Pick mode active: click map to set node position, drag marker for precision.'
                  : 'Pick mode paused.')
              : 'Edit node details and save changes.'}
          </div>
          <div class="form-grid two-col">
            <label class="field">
              <span>Name</span>
              <input class="input" bind:value={nodeForm.name} />
            </label>
            <label class="field">
              <span>Type</span>
              <Select2
                bind:value={nodeForm.node_type}
                options={nodeTypeOptions}
                width="100%"
                placeholder="Select node type"
                searchPlaceholder="Search type..."
                noResultsText="No type found"
              />
            </label>
            <label class="field">
              <span>Status</span>
              <select class="input" bind:value={nodeForm.status}>
                <option value="active">active</option>
                <option value="inactive">inactive</option>
                <option value="maintenance">maintenance</option>
              </select>
            </label>
            {#if !editingNodeId}
              <label class="field">
                <span>Latitude</span>
                <input class="input" type="number" step="0.000001" bind:value={nodeForm.lat} />
              </label>
              <label class="field">
                <span>Longitude</span>
                <input class="input" type="number" step="0.000001" bind:value={nodeForm.lng} />
              </label>
            {:else}
              <div class="field field-full node-edit-location-hint">
                <span>Location</span>
                <div class="hint-card">
                  Marker is draggable on map. Drag marker to update node position.
                  <div class="hint-coord">{nodeForm.lat}, {nodeForm.lng}</div>
                </div>
              </div>
            {/if}
          </div>
          <div class="node-panel-actions">
            <button class="btn ghost" type="button" onclick={closeNodeModal} disabled={savingNode}>Cancel</button>
            <button class="btn" type="button" onclick={() => void submitNode()} disabled={savingNode}>
              {savingNode ? 'Saving...' : editingNodeId ? 'Update Node' : 'Save Node'}
            </button>
          </div>
        </div>
      {/if}

      {#if linkPickMode}
        <div class="map-link-draw-controls">
          {#if linkPickDrawMode === 'path'}
            <button class="btn ghost btn-xs" type="button" onclick={undoLinkPathPoint} disabled={linkPathBendPoints.length === 0}>
              <Icon name="arrow-left" size={14} />
              Undo
            </button>
          {/if}
          <button class="btn ghost btn-xs danger" type="button" onclick={cancelLinkPicking}>
            <Icon name="x-circle" size={14} />
            Cancel
          </button>
        </div>
      {/if}
    </div>
  </div>

  <div class="manager-wrap">
    <div class="manager-header">
      <div class="manager-tabs">
        <button class:active={selectedTab === 'nodes'} onclick={() => (selectedTab = 'nodes')}>Nodes</button>
        <button class:active={selectedTab === 'links'} onclick={() => (selectedTab = 'links')}>Links</button>
        <button class:active={selectedTab === 'zones'} onclick={() => (selectedTab = 'zones')}>Zones</button>
        <button class:active={selectedTab === 'bindings'} onclick={() => (selectedTab = 'bindings')}>
          Zone Bindings
        </button>
      </div>

      <div class="manager-actions">
        {#if selectedTab === 'nodes'}
          <button class="btn" type="button" onclick={openCreateNodeModal}>
            <Icon name="plus" size={14} />
            Add Node
          </button>
        {:else if selectedTab === 'links'}
          <button class="btn" type="button" onclick={openCreateLinkModal}>
            <Icon name="plus" size={14} />
            Add Link
          </button>
        {:else if selectedTab === 'zones'}
          <button class="btn" type="button" onclick={openCreateZoneModal}>
            <Icon name="plus" size={14} />
            Add Zone
          </button>
        {/if}
      </div>
    </div>

    {#if selectedTab === 'nodes'}
      <div class="table-wrap">
        <div class="table-top"><strong>{nodeRows.length}</strong> nodes in viewport</div>
        <table class="table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Status</th>
              <th>Coordinates</th>
              <th class="right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#if nodeRows.length === 0}
              <tr><td colspan="5" class="empty">No nodes</td></tr>
            {:else}
              {#each nodeRows as row}
                <tr>
                  <td>{row.name}</td>
                  <td>{row.node_type}</td>
                  <td>{row.status}</td>
                  <td>{row.lat.toFixed(6)}, {row.lng.toFixed(6)}</td>
                  <td class="right">
                    <button class="btn ghost btn-xs" onclick={() => openEditNodeModal(row)}>Edit</button>
                    <button class="btn ghost btn-xs danger" onclick={() => openDeleteConfirm('node', row.id, row.name)} disabled={deletingId === row.id}>Delete</button>
                  </td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    {/if}

    {#if selectedTab === 'links'}
  <div class="table-wrap">
        <div class="table-top"><strong>{linkRows.length}</strong> links in viewport</div>
        <table class="table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Status</th>
              <th>Health</th>
              <th>Endpoints</th>
              <th class="right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#if linkRows.length === 0}
              <tr><td colspan="6" class="empty">No links</td></tr>
            {:else}
              {#each linkRows as row}
                {@const health = computeLinkHealth(row)}
                <tr>
                  <td>{row.name}</td>
                  <td>{row.link_type}</td>
                  <td>{row.status}</td>
                  <td>
                    <span class={`health-pill ${health.tone}`}>{health.score}</span>
                  </td>
                  <td>{row.from_node_id || '-'} → {row.to_node_id || '-'}</td>
                  <td class="right">
                    <button class="btn ghost btn-xs" onclick={() => openEditLinkModal(row)}>Edit</button>
                    <button class="btn ghost btn-xs danger" onclick={() => openDeleteConfirm('link', row.id, row.name)} disabled={deletingId === row.id}>Delete</button>
                  </td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    {/if}

    {#if selectedTab === 'zones'}
      <div class="table-wrap">
        <div class="table-top"><strong>{zoneRows.length}</strong> zones in viewport</div>
        <table class="table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Status</th>
              <th class="right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#if zoneRows.length === 0}
              <tr><td colspan="4" class="empty">No zones</td></tr>
            {:else}
              {#each zoneRows as row}
                <tr>
                  <td>{row.name}</td>
                  <td>{row.zone_type}</td>
                  <td>{row.status}</td>
                  <td class="right">
                    <button class="btn ghost btn-xs" onclick={() => openEditZoneModal(row)}>Edit</button>
                    <button class="btn ghost btn-xs danger" onclick={() => openDeleteConfirm('zone', row.id, row.name)} disabled={deletingId === row.id}>Delete</button>
                  </td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    {/if}

    {#if selectedTab === 'bindings'}
      <div class="bindings-wrap">
        <div class="binding-form">
          <div class="control">
            <label for="zone-id">Zone</label>
            <select id="zone-id" class="input" bind:value={selectedZoneId}>
              <option value="">Select zone</option>
              {#each zoneRows as z}
                <option value={z.id}>{z.name}</option>
              {/each}
            </select>
          </div>
          <div class="control">
            <label for="node-id">Node</label>
            <select id="node-id" class="input" bind:value={bindingForm.node_id} disabled={!selectedZoneId}>
              <option value="">Select node</option>
              {#each nodeRows as n}
                <option value={n.id}>{n.name}</option>
              {/each}
            </select>
          </div>
          <div class="control">
            <label for="binding-weight">Weight</label>
            <input id="binding-weight" class="input" type="number" min="1" bind:value={bindingForm.weight} />
          </div>
          <label class="toggle"><input type="checkbox" bind:checked={bindingForm.is_primary} /> <span>Primary</span></label>
          <button class="btn" type="button" onclick={() => void createZoneBinding()} disabled={!selectedZoneId || savingBinding}>
            <Icon name="plus" size={14} />
            Add Binding
          </button>
        </div>

        <div class="table-wrap">
          <div class="table-top"><strong>{zoneBindings.length}</strong> bindings</div>
          <table class="table">
            <thead>
              <tr>
                <th>Zone</th>
                <th>Node</th>
                <th>Primary</th>
                <th>Weight</th>
                <th class="right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {#if loadingManager}
                <tr><td colspan="5" class="empty">Loading...</td></tr>
              {:else if zoneBindings.length === 0}
                <tr><td colspan="5" class="empty">No bindings</td></tr>
              {:else}
                {#each zoneBindings as row}
                  <tr>
                    <td>{zoneRows.find((z) => z.id === row.zone_id)?.name || row.zone_id}</td>
                    <td>{nodeRows.find((n) => n.id === row.node_id)?.name || row.node_id}</td>
                    <td>{row.is_primary ? 'Yes' : 'No'}</td>
                    <td>{row.weight}</td>
                    <td class="right">
                      <button class="btn ghost btn-xs danger" onclick={() => openDeleteConfirm('binding', row.id)} disabled={deletingId === row.id}>Delete</button>
                    </td>
                  </tr>
                {/each}
              {/if}
            </tbody>
          </table>
        </div>
      </div>
    {/if}
  </div>
</div>

<Modal
  show={showLinkModal}
  title={editingLinkId ? 'Edit Link' : 'Add Link'}
  width="860px"
  onclose={() => !savingLink && closeLinkModal()}
>
  {#if !editingLinkId}
    <div class="link-pick-toolbar">
      <div class="link-pick-mode">
        <button
          class={`mode-btn ${linkPickDrawMode === 'quick' ? 'active' : ''}`}
          type="button"
          onclick={() => setLinkPickDrawMode('quick')}
        >
          Quick
        </button>
        <button
          class={`mode-btn ${linkPickDrawMode === 'path' ? 'active' : ''}`}
          type="button"
          onclick={() => setLinkPickDrawMode('path')}
        >
          Draw Path
        </button>
      </div>
      <button
        class={`btn ghost btn-xs ${linkPickMode ? 'active' : ''}`}
        type="button"
        onclick={toggleLinkPickMode}
      >
        <Icon name="map-pin" size={14} />
        {linkPickMode
          ? `Picking ${linkPickStep === 'from' ? 'source' : 'destination'}...`
          : linkPickDrawMode === 'quick'
            ? 'Pick Endpoints on Map'
            : 'Draw Path on Map'}
      </button>
      {#if linkPickMode && linkPickDrawMode === 'path'}
        <button class="btn ghost btn-xs" type="button" onclick={undoLinkPathPoint} disabled={linkPathBendPoints.length === 0}>
          <Icon name="arrow-left" size={14} />
          Undo
        </button>
        <button class="btn ghost btn-xs" type="button" onclick={clearLinkPathPoints} disabled={linkPathBendPoints.length === 0}>
          <Icon name="x-circle" size={14} />
          Clear
        </button>
      {/if}
    </div>
    {#if linkPickMode}
      <div class="link-pick-hint">
        {#if linkPickDrawMode === 'quick'}
          Quick: click source node then destination node.
        {:else if linkPickStep === 'from'}
          Draw Path: click source node.
        {:else}
          Draw Path: click map to add bend points, then click destination node.
        {/if}
      </div>
    {/if}
  {/if}
  <div class="form-grid two-col">
    <label class="field">
      <span>Name</span>
      <input class="input" bind:value={linkForm.name} />
    </label>
    <label class="field">
      <span>Type</span>
      <Select2
        bind:value={linkForm.link_type}
        options={linkTypeOptions}
        width="100%"
        placeholder="Select link type"
        searchPlaceholder="Search type..."
        noResultsText="No type found"
      />
    </label>
    <label class="field">
      <span>Status</span>
      <Select2
        bind:value={linkForm.status}
        options={linkStatusOptions}
        width="100%"
        placeholder="Select status"
        searchPlaceholder="Search status..."
        noResultsText="No status found"
      />
    </label>
    <label class="field">
      <span>From Node</span>
      <select class="input" bind:value={linkForm.from_node_id}>
        <option value="">Select node</option>
        {#each nodeRows as n}
          <option value={n.id}>{n.name}</option>
        {/each}
      </select>
    </label>
    <label class="field">
      <span>To Node</span>
      <select class="input" bind:value={linkForm.to_node_id}>
        <option value="">Select node</option>
        {#each nodeRows as n}
          <option
            value={n.id}
            disabled={
              n.id === linkForm.from_node_id ||
              hasExistingLinkBetweenNodes(linkForm.from_node_id, n.id, editingLinkId)
            }
          >
            {n.name}
          </option>
        {/each}
      </select>
    </label>
    <label class="field">
      <span>Priority</span>
      <input class="input" type="number" min="1" bind:value={linkForm.priority} />
    </label>
    <div class="field field-full link-type-helper">
      <Icon name="info" size={14} />
      <span>{linkFieldConfig.helper}</span>
    </div>
    <label class="field">
      <span>{linkFieldConfig.capacityLabel}</span>
      <input class="input" type="number" min="0" step="0.01" bind:value={linkForm.capacity_mbps} />
    </label>
    <label class="field">
      <span>{linkFieldConfig.utilizationLabel}</span>
      <input class="input" type="number" min="0" max="100" step="0.01" bind:value={linkForm.utilization_pct} />
    </label>
    {#if linkFieldConfig.showLoss}
      <label class="field">
        <span>{linkFieldConfig.lossLabel}</span>
        <input class="input" type="number" step="0.01" bind:value={linkForm.loss_db} />
      </label>
    {/if}
    <label class="field">
      <span>{linkFieldConfig.latencyLabel}</span>
      <input class="input" type="number" min="0" step="0.01" bind:value={linkForm.latency_ms} />
    </label>
    <label class="field field-full">
      <span>Geometry (GeoJSON LineString)</span>
      <textarea class="input textarea" rows="7" bind:value={linkForm.geometryText}></textarea>
    </label>
  </div>
  <div class="inline-actions">
    <button class="btn ghost btn-xs" type="button" onclick={useLinkFromNodePoints}>Use straight line from selected nodes</button>
  </div>
  {#snippet footer()}
    <button class="btn ghost" type="button" onclick={closeLinkModal} disabled={savingLink}>Cancel</button>
    <button class="btn" type="button" onclick={() => void submitLink()} disabled={savingLink}>
      {savingLink ? 'Saving...' : 'Save'}
    </button>
  {/snippet}
</Modal>

<Modal
  show={showZoneModal}
  title={editingZoneId ? 'Edit Zone' : 'Add Zone'}
  width="860px"
  onclose={() => !savingZone && (showZoneModal = false)}
>
  <div class="form-grid two-col">
    <label class="field">
      <span>Name</span>
      <input class="input" bind:value={zoneForm.name} />
    </label>
    <label class="field">
      <span>Type</span>
      <input class="input" bind:value={zoneForm.zone_type} />
    </label>
    <label class="field">
      <span>Status</span>
      <select class="input" bind:value={zoneForm.status}>
        <option value="active">active</option>
        <option value="inactive">inactive</option>
      </select>
    </label>
    <label class="field">
      <span>Priority</span>
      <input class="input" type="number" min="1" bind:value={zoneForm.priority} />
    </label>
    <label class="field field-full">
      <span>Geometry (GeoJSON Polygon/MultiPolygon)</span>
      <textarea class="input textarea" rows="9" bind:value={zoneForm.geometryText}></textarea>
    </label>
  </div>
  {#snippet footer()}
    <button class="btn ghost" type="button" onclick={() => (showZoneModal = false)} disabled={savingZone}>Cancel</button>
    <button class="btn" type="button" onclick={() => void submitZone()} disabled={savingZone}>
      {savingZone ? 'Saving...' : 'Save'}
    </button>
  {/snippet}
</Modal>

<ConfirmDialog
  show={showDeleteConfirm}
  title={deleteConfirmTitle}
  message={deleteConfirmMessage}
  confirmText="Delete"
  cancelText="Cancel"
  type="danger"
  loading={Boolean(deletingId)}
  onconfirm={() => void confirmDeleteAction()}
  oncancel={() => {
    showDeleteConfirm = false;
    deleteTargetType = null;
    deleteTargetId = '';
  }}
/>

<style>
  .page-content {
    padding: 28px;
    max-width: 1460px;
    margin: 0 auto;
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
    font-weight: 800;
    cursor: pointer;
    text-decoration: none;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .filters-wrap {
    margin-bottom: 12px;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }

  .stat-card {
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--bg-card) 86%, #16213f 14%) 0%,
        var(--bg-card) 100%
      );
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 14px 14px 12px;
    box-shadow: inset 0 1px 0 rgba(148, 163, 184, 0.08);
  }

  .stat-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-secondary);
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .stat-value {
    margin-top: 10px;
    font-size: 1.6rem;
    font-weight: 950;
    color: var(--text-primary);
  }

  .tone-ok {
    border-color: color-mix(in srgb, #1fbf75 55%, var(--border-color));
  }

  .tone-warn {
    border-color: color-mix(in srgb, #ffcc66 55%, var(--border-color));
  }

  .toolbar-wrap {
    display: grid;
    gap: 8px;
  }

  .map-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px 12px;
    background: var(--bg-card);
    box-shadow: inset 0 1px 0 rgba(148, 163, 184, 0.08);
  }

  :global(.nm-location-ctrl) {
    font-size: 17px;
    line-height: 1;
    color: var(--text-secondary);
  }

  :global(.nm-location-ctrl:hover:not(:disabled)) {
    color: var(--text-primary);
  }

  :global(.nm-location-ctrl.active) {
    color: #3f8cff;
  }

  :global(.nm-location-ctrl.loading) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .layer-toggles {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.86rem;
    color: var(--text-secondary);
  }

  .map-shell {
    position: relative;
    min-height: 520px;
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
    background: var(--bg-card);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.2);
  }

  .map-link-draw-controls {
    position: absolute;
    top: 14px;
    right: 58px;
    z-index: 8;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 8px;
    border-radius: 10px;
    border: 1px solid var(--border-color, #24304a);
    background: var(--panel-bg, #0f1422);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  }

  .map-view-switch {
    position: absolute;
    top: 12px;
    left: 12px;
    z-index: 8;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px;
    border-radius: 999px;
    border: 1px solid var(--border-color, #24304a);
    background: var(--panel-bg, #0f1422);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.24);
  }

  .switch-btn {
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary, #94a3b8);
    border-radius: 999px;
    min-width: 44px;
    height: 30px;
    padding: 0 10px;
    font-size: 0.74rem;
    font-weight: 700;
    letter-spacing: 0.01em;
    cursor: pointer;
  }

  .switch-btn:hover {
    color: #e2e8f0;
    background: color-mix(in srgb, #334155 30%, transparent);
  }

  .switch-btn.active {
    color: #f8fafc;
    border-color: color-mix(in srgb, var(--color-primary) 42%, #60a5fa);
    background: color-mix(in srgb, var(--color-primary) 28%, #0b1225);
  }

  .node-create-panel {
    position: absolute;
    top: 12px;
    left: 12px;
    width: min(520px, calc(100% - 24px));
    max-height: calc(100% - 24px);
    overflow: visible;
    z-index: 1200;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: #0f1527;
    padding: 10px;
    display: grid;
    gap: 10px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
  }

  .node-panel-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .node-create-panel :global(.select-container) {
    position: relative;
    z-index: 1201;
  }

  .node-create-panel :global(.dropdown-menu) {
    z-index: 1300;
  }

  .map-canvas {
    width: 100%;
    min-height: 520px;
  }

  .map-loading {
    position: absolute;
    top: 10px;
    left: 10px;
    z-index: 2;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 6px 10px;
    background: var(--bg-card);
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .map-unavailable {
    min-height: 520px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--text-secondary);
    padding: 20px;
    text-align: left;
  }

  .map-unavailable-title {
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .map-unavailable-sub {
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .location-error {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: #fbbf24;
  }

  .manager-wrap {
    margin-top: 14px;
    border: 1px solid var(--border-color);
    border-radius: 16px;
    background: var(--bg-card);
    overflow: hidden;
  }

  .manager-header {
    padding: 10px 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    border-bottom: 1px solid var(--border-color);
  }

  .manager-tabs {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .manager-tabs button {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 94%, #0a1020 6%);
    color: var(--text-secondary);
    border-radius: 10px;
    padding: 8px 10px;
    font-weight: 700;
    cursor: pointer;
  }

  .manager-tabs button.active {
    color: var(--text-primary);
    border-color: color-mix(in srgb, var(--color-primary) 50%, var(--border-color));
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-primary) 22%, #0b1225 78%) 0%,
      color-mix(in srgb, var(--color-primary) 14%, #0b1225 86%) 100%
    );
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.08),
      0 4px 16px rgba(56, 96, 255, 0.2);
  }

  .manager-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .table-wrap {
    overflow-x: auto;
  }

  .table-top {
    padding: 10px 12px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .table {
    width: 100%;
    border-collapse: collapse;
    min-width: 720px;
  }

  .table th,
  .table td {
    text-align: left;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-color);
    font-size: 0.88rem;
  }

  .table th {
    color: var(--text-secondary);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.72rem;
  }

  .table .right {
    text-align: right;
    white-space: nowrap;
  }

  .table .empty {
    text-align: center;
    color: var(--text-secondary);
    padding: 18px 12px;
  }

  .btn-xs {
    padding: 6px 10px;
    font-size: 0.78rem;
    border-radius: 9px;
  }

  .btn.danger {
    color: #fca5a5;
    border-color: color-mix(in srgb, #ef4444 55%, var(--border-color));
  }

  .bindings-wrap {
    display: grid;
    gap: 10px;
    padding: 10px;
  }

  .binding-form {
    display: grid;
    gap: 10px;
    grid-template-columns: repeat(4, minmax(0, 1fr)) auto auto;
    align-items: end;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 10px;
  }

  .control {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .control label {
    font-size: 0.78rem;
    color: #cbd5e1;
    font-weight: 700;
  }

  .input {
    width: 100%;
    border: 1px solid #334155;
    border-radius: 10px;
    background: #111827;
    color: #e5e7eb;
    padding: 10px 12px;
    font-size: 0.9rem;
    outline: none;
  }

  .input:focus {
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-color));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 22%, transparent);
  }

  .form-grid {
    display: grid;
    gap: 12px;
  }

  .form-grid.two-col {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field > span {
    font-size: 0.8rem;
    color: #cbd5e1;
    font-weight: 700;
  }

  .field-full {
    grid-column: 1 / -1;
  }

  .node-edit-location-hint .hint-card {
    border: 1px solid #334155;
    border-radius: 10px;
    background: #0b1322;
    color: #cbd5e1;
    padding: 10px 12px;
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .node-edit-location-hint .hint-coord {
    margin-top: 6px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.8rem;
    color: #93c5fd;
  }

  .textarea {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    min-height: 130px;
    resize: vertical;
  }

  .inline-actions {
    margin-top: 10px;
    display: flex;
    justify-content: flex-end;
  }

  .link-pick-toolbar {
    margin-bottom: 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .link-pick-mode {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    overflow: hidden;
    background: #0f172a;
  }

  .mode-btn {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    padding: 7px 10px;
    font-weight: 700;
    cursor: pointer;
  }

  .mode-btn.active {
    color: #e5e7eb;
    background: color-mix(in srgb, var(--color-primary) 28%, #0b1225);
  }

  .link-pick-hint {
    margin-bottom: 6px;
    padding: 8px 10px;
    border: 1px solid color-mix(in srgb, #3f8cff 45%, var(--border-color));
    border-radius: 10px;
    background: #0b1322;
    color: #dbe7ff;
    font-size: 0.82rem;
    line-height: 1.4;
  }

  .link-pick-toolbar .btn.active {
    border-color: color-mix(in srgb, var(--color-primary) 65%, #60a5fa);
    background: color-mix(in srgb, var(--color-primary) 22%, transparent);
    color: #dbeafe;
  }

  .link-type-helper {
    display: flex;
    align-items: center;
    gap: 8px;
    border: 1px solid color-mix(in srgb, #60a5fa 36%, var(--border-color));
    background: #0b1322;
    color: #cfe2ff;
    border-radius: 10px;
    padding: 8px 10px;
    font-size: 0.82rem;
    line-height: 1.4;
  }

  .health-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid transparent;
    font-weight: 800;
    font-size: 0.75rem;
    letter-spacing: 0.02em;
  }

  .health-pill.good {
    color: #16a34a;
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.35);
  }

  .health-pill.warn {
    color: #d97706;
    background: rgba(245, 158, 11, 0.13);
    border-color: rgba(245, 158, 11, 0.35);
  }

  .health-pill.bad {
    color: #dc2626;
    background: rgba(239, 68, 68, 0.12);
    border-color: rgba(239, 68, 68, 0.35);
  }

  .panel-head {
    margin-bottom: 4px;
    padding: 10px 12px;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, #3f8cff 40%, var(--border-color));
    background: #0b1322;
    color: #dbe7ff;
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .panel-title {
    font-size: 0.95rem;
    font-weight: 900;
    color: #f8fafc;
    margin-bottom: 4px;
  }

  .node-create-panel .btn.ghost {
    color: #e2e8f0;
    border-color: #475569;
    background: #0b1322;
  }

  .node-create-panel .btn.ghost:hover {
    background: #131d30;
  }

  :global(.my-location-dot) {
    width: 16px;
    height: 16px;
    border-radius: 999px;
    background: #2d7fff;
    border: 2px solid #ffffff;
    box-shadow:
      0 0 0 4px rgba(45, 127, 255, 0.24),
      0 4px 12px rgba(0, 0, 0, 0.35);
  }

  :global(.maplibregl-popup-content) {
    background: #0f172a;
    color: #e2e8f0;
    border: 1px solid #334155;
    border-radius: 10px;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35);
    padding: 10px 12px;
    min-width: 260px;
  }

  :global(.maplibregl-popup-tip) {
    border-top-color: #0f172a !important;
    border-bottom-color: #0f172a !important;
  }

  :global(.maplibregl-popup-close-button) {
    color: #cbd5e1;
  }

  :global(.maplibregl-popup-close-button:hover) {
    background: #1e293b;
    color: #f8fafc;
  }

  :global(.nm-popup-card) {
    display: grid;
    gap: 8px;
  }

  :global(.nm-popup-head) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  :global(.nm-popup-title) {
    font-size: 0.95rem;
    font-weight: 900;
    color: #f8fafc;
    letter-spacing: 0.01em;
  }

  :global(.nm-popup-badge) {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 3px 8px;
    font-size: 0.7rem;
    font-weight: 900;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border: 1px solid transparent;
  }

  :global(.nm-popup-badge.ok) {
    color: #22c55e;
    background: rgba(34, 197, 94, 0.14);
    border-color: rgba(34, 197, 94, 0.35);
  }

  :global(.nm-popup-badge.warn) {
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.14);
    border-color: rgba(245, 158, 11, 0.35);
  }

  :global(.nm-popup-badge.muted) {
    color: #94a3b8;
    background: rgba(148, 163, 184, 0.14);
    border-color: rgba(148, 163, 184, 0.3);
  }

  :global(.nm-popup-grid) {
    display: grid;
    grid-template-columns: 86px 1fr;
    gap: 6px 10px;
  }

  :global(.nm-popup-label) {
    color: #94a3b8;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 800;
  }

  :global(.nm-popup-value) {
    color: #e2e8f0;
    font-size: 0.83rem;
    font-weight: 600;
  }

  :global(.nm-popup-value.mono) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.78rem;
  }

  :global(.nm-popup-actions) {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid rgba(148, 163, 184, 0.2);
  }

  :global(.nm-popup-btn) {
    height: 30px;
    padding: 0 10px;
    border-radius: 8px;
    border: 1px solid #475569;
    background: #0b1322;
    color: #e2e8f0;
    font-size: 0.78rem;
    font-weight: 700;
    cursor: pointer;
  }

  :global(.nm-popup-btn:hover) {
    background: #131d30;
  }

  :global(.nm-popup-btn.primary) {
    border-color: color-mix(in srgb, var(--color-primary) 65%, #475569);
    background: color-mix(in srgb, var(--color-primary) 22%, #0b1322);
    color: #eef2ff;
  }

  :global(.nm-popup-btn.danger) {
    border-color: color-mix(in srgb, #ef4444 58%, #7f1d1d);
    background: color-mix(in srgb, #ef4444 18%, #0b1322);
    color: #fecaca;
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 18px;
    }

    .page-content :global(.network-page-title) {
      font-size: 1.35rem;
    }

    .page-content :global(.network-page-subtitle) {
      font-size: 0.9rem;
    }

    .page-content :global(.network-page-actions) {
      width: 100%;
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }

    .page-content :global(.network-page-actions .btn) {
      width: 100%;
      justify-content: center;
    }

    .page-content :global(.network-filter-panel) {
      grid-template-columns: 1fr;
      padding: 10px;
    }

    .page-content :global(.network-filter-panel .control-actions .label) {
      display: none;
    }

    .map-toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .layer-toggles {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px 10px;
      align-items: start;
    }

    .toggle {
      min-height: 34px;
    }

    .stats {
      justify-content: flex-start;
    }

    .map-shell,
    .map-canvas {
      min-height: 400px;
    }

    .node-create-panel {
      left: 8px;
      right: 8px;
      top: 8px;
      width: auto;
      max-height: calc(100% - 16px);
    }

  }

  @media (max-width: 560px) {
    .page-content :global(.network-page-actions) {
      grid-template-columns: 1fr;
    }

    .layer-toggles {
      grid-template-columns: 1fr;
    }

    .manager-header {
      flex-direction: column;
      align-items: stretch;
    }

    .binding-form {
      grid-template-columns: 1fr;
    }

    .form-grid.two-col {
      grid-template-columns: 1fr;
    }

    .map-shell,
    .map-canvas {
      min-height: 340px;
    }
  }
</style>
