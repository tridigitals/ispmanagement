<script lang="ts">
  import type { Geometry } from 'geojson';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onDestroy, onMount, type Component } from 'svelte';
  import { t } from 'svelte-i18n';
  import { customers } from '$lib/api/customers';
  import { networkAssets } from '$lib/api/networkAssets';
  import type { NetworkAssetListItem } from '$lib/api/types';
  import { can, tenant, user } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import {
    buildLinkDraftForm,
    buildZoneDraftForm,
    createNetworkZoneBinding,
    deleteNetworkLink,
    deleteNetworkNode,
    deleteNetworkZone,
    deleteNetworkZoneBinding,
    loadNetworkZoneBindings,
    saveNetworkLink,
    saveNetworkNode,
    saveNetworkZone,
  } from '$lib/components/network/networkMapActions';
  import {
    buildTopologyAssetAutoLinkFeatureCollection,
    buildTopologyAssetRows,
    topologyAssetsToFeatureCollection,
    type TopologyAssetRow,
  } from '$lib/components/network/networkMapAssets';
  import {
    buildTopologyAssetConnectionOperations,
    buildTopologyAssetConnectDraft,
    findTopologyAssetNodeId,
  } from '$lib/components/network/networkMapAssetConnect';
  import {
    buildDefaultLineGeometry,
    buildDeleteConfirmCopy,
    currentDraftPathCoords,
    hasExistingLinkBetweenNodes,
  } from '$lib/components/network/networkMapInteractionUtils';
  import { snapshotMapFeature } from '$lib/components/network/networkMapEventSnapshot';
  import {
    removeCrud,
    submitLinkCrud,
    submitNodeCrud,
    submitZoneCrud,
  } from '$lib/components/network/networkMapCrud';
  import {
    applyPickedNodeMarker,
    buildDefaultZoneGeometry,
    buildLinkDraftPreviewCollections,
    buildLinkGeometryDraftText,
    clearDraftNodeMarker,
  } from '$lib/components/network/networkMapDrafts';
  import {
    buildConnectFromNodeResult,
    buildHandlePickedLinkNodeResult,
    buildSetLinkDrawModeResult,
    buildStraightLinkGeometryText,
    buildToggleLinkPickResult,
    createEditLinkForm,
    createLinkForm,
    resolveLinkGeometryTextForSubmit,
    type LinkPickDrawMode,
  } from '$lib/components/network/networkMapLinkPicking';
  import {
    applyCachedMapData,
    applyFetchedMapData,
    buildMapDataCacheKey,
    fetchNetworkMapData,
    getCachedMapData,
    getTopologySyncStrategy,
    resolveNetworkMapFetchBbox,
    setCachedMapData,
    shouldFetchRouterOverlay,
    syncTopologyAssetsIfNeeded,
    type NetworkMapCacheEntry,
  } from '$lib/components/network/networkMapData';
  import {
    emitInstallationRefreshSignal,
    emitWorkOrderUpdatedToParent,
    resolveInstallationTargetMarker,
  } from '$lib/components/network/networkMapInstallation';
  import { parseNetworkAssetMapTarget } from '../assets/networkAssetMapNavigation';
  import {
    asNumber,
    customersToFeatureCollection,
    filterRoutersForOverlay,
    getLinkFieldConfig,
    isCustomerNodeType,
    isSystemManagedNode,
    linkStatusOptions,
    linkTypeOptions,
    nodeTypeOptions,
    parseGeometryText,
    prettyGeometry,
    systemManagedNodeSourceLabel,
    type LinkFieldConfig,
    type NMLink,
    type NMNode,
    type NMRouter,
    type NMZone,
  } from '$lib/components/network/networkMapUtils';
  import {
    applyNetworkMapWorkspaceDefaults,
    buildNetworkMapWorkspaceDefaults,
    buildSelectedMapObject,
    createNetworkMapWorkspaceState,
    type NetworkMapWorkspaceState,
    type NetworkMapWorkspaceCapabilities,
  } from '$lib/components/network/networkMapWorkspaceState';
  import { type NetworkMapSearchResultItem } from '$lib/components/network/networkMapInsights';
  import {
    buildNetworkMapOverviewSearchGroups,
    countNetworkMapSearchResults,
  } from '$lib/components/network/networkMapOverviewModel';
  import {
    loadNetworkMapChromeModules,
    loadNetworkMapDialogModules,
    loadNetworkMapInteractionModule,
    loadNetworkMapPopupModule,
    loadNetworkMapWorkspaceModule,
    type NetworkMapInteractionModule,
    type NetworkMapWorkspaceModule,
  } from '$lib/components/network/networkMapUiModules';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import NetworkMapSearchBar from '$lib/components/network/NetworkMapSearchBar.svelte';
  import { canAccessNetworkMap } from '$lib/utils/adminNetworkAccess';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type MaplibreModule = typeof import('maplibre-gl');
  type MapInstance = import('maplibre-gl').Map;
  type NetworkMapQuickMode = 'all' | 'issues' | 'customers' | 'services' | 'topology' | 'field';

  const SOURCE_NODES = 'nm-nodes';
  const SOURCE_CUSTOMERS = 'nm-customers';
  const SOURCE_LINKS = 'nm-links';
  const SOURCE_ZONES = 'nm-zones';
  const SOURCE_ROUTERS = 'nm-routers';
  const SOURCE_TOPOLOGY_ASSETS = 'nm-topology-assets';
  const SOURCE_TOPOLOGY_ASSET_LINKS = 'nm-topology-asset-links';
  const SOURCE_LINK_DRAFT = 'nm-link-draft';
  const SOURCE_LINK_DRAFT_POINTS = 'nm-link-draft-points';
  const SOURCE_SELECTION_POINTS = 'nm-selection-points';
  const SOURCE_SELECTION_LINES = 'nm-selection-lines';
  const SOURCE_SELECTION_ZONES = 'nm-selection-zones';

  let mapEl = $state<HTMLDivElement | null>(null);
  let map = $state<MapInstance | null>(null);
  let maplibre = $state<MaplibreModule | null>(null);
  let interactionModule = $state<NetworkMapInteractionModule | null>(null);
  let workspaceModule = $state<NetworkMapWorkspaceModule | null>(null);
  let mapReady = $state(false);
  let mapUnavailable = $state(false);
  let mapErrorMessage = $state('');
  let loading = $state(true);
  let refreshing = $state(false);
  let syncingAssetNodes = $state(false);
  let OverviewComponent = $state<Component | null>(null);
  let FloatingControlsComponent = $state<Component | null>(null);
  let NodePanelComponent = $state<Component | null>(null);
  let LinkModalComponent = $state<Component | null>(null);
  let ZoneModalComponent = $state<Component | null>(null);
  let ConfirmDialogComponent = $state<Component | null>(null);

  let nodesVisible = $state(true);
  let linksVisible = $state(true);
  let zonesVisible = $state(true);
  let routersVisible = $state(true);
  let customersVisible = $state(true);
  let topologyAssetsVisible = $state(true);
  let viewMode = $state<'standard' | 'satellite'>('standard');
  let controlsHidden = $state(true);

  let q = $state('');
  let workspaceSearchQuery = $state('');
  let workspaceSearchOpen = $state(false);
  let status = $state('');
  let kind = $state('');
  let quickMode = $state<NetworkMapQuickMode>('all');

  let nodeCount = $state(0);
  let linkCount = $state(0);
  let zoneCount = $state(0);
  let nodeRows = $state<NMNode[]>([]);
  let linkRows = $state<NMLink[]>([]);
  let zoneRows = $state<NMZone[]>([]);
  let routerRows = $state<NMRouter[]>([]);
  let topologyAssetRows = $state<TopologyAssetRow[]>([]);
  let topologyAssetItems = $state<NetworkAssetListItem[]>([]);
  let customerRows = $state<NMNode[]>([]);
  let serviceRows = $state<NMNode[]>([]);
  let savingNode = $state(false);
  let savingLink = $state(false);
  let savingZone = $state(false);
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
  let linkSnapToNodeEnabled = $state(true);
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
  let activeAssetConnectSourceId = $state<string | null>(null);

  let workspaceState = $state<NetworkMapWorkspaceState>(
    createNetworkMapWorkspaceState({
      canManageTopology: false,
      canReadCustomers: false,
      canReadWorkOrders: false,
      canReadNetworkNoc: false,
      canReadRouterInventory: false,
    }),
  );

  let refreshDebounce: ReturnType<typeof setTimeout> | null = null;
  let freshnessTimer: ReturnType<typeof setInterval> | null = null;
  let lastRequestId = 0;
  let installationTargetMarker: import('maplibre-gl').Marker | null = null;
  let installationTargetCoord: [number, number] | null = null;
  let installationTargetResolved = false;
  let activeNodePopup: import('maplibre-gl').Popup | null = null;
  let activeDataAbortController: AbortController | null = null;
  let backgroundAssetSyncPromise: Promise<boolean> | null = null;
  let didInitialFitToMarkers = false;
  let initialExtentLoaded = false;
  let assetFocusApplied = false;
  let lastAssetSyncAt = 0;
  let lastMapDataLoadedAt = $state(0);
  let lastMapDataSource = $state<'live' | 'cache' | 'none'>('none');
  let currentTimeMs = $state(Date.now());
  const dataCache = new Map<string, NetworkMapCacheEntry>();
  const dataCacheTtlMs = 20_000;
  const dataCacheMaxEntries = 40;
  const assetSyncTtlMs = 45_000;
  const topologyAssetsCacheTtlMs = 60_000;
  const mapTilerKey = (import.meta.env.VITE_MAPTILER_KEY as string | undefined)?.trim();
  let lastTopologyAssetsLoadedAt = 0;
  let refreshingTopologyAssets = false;
  const hasHiResSatellite = Boolean(mapTilerKey);
  const standardMaxZoom = 19;
  const satelliteMaxZoom = hasHiResSatellite ? 21 : 18;

  const canManageTopology = $derived($can('manage', 'network_topology'));
  const canReadRouterInventory = $derived(
    $can('read', 'router_inventory') || $can('manage', 'router_inventory'),
  );
  const workspaceCapabilities = $derived.by<NetworkMapWorkspaceCapabilities>(() => ({
    canManageTopology,
    canReadCustomers: $can('read', 'customers') || $can('manage', 'customers'),
    canReadWorkOrders: $can('read', 'work_orders') || $can('manage', 'work_orders'),
    canReadNetworkNoc: $can('read', 'network_noc') || $can('manage', 'network_noc'),
    canReadRouterInventory,
  }));
  const workspaceDefaults = $derived.by(() =>
    buildNetworkMapWorkspaceDefaults(workspaceCapabilities),
  );
  const mapDataFreshnessLabel = $derived.by(() => {
    if (!lastMapDataLoadedAt) {
      return (
        $t('admin.network.map.workspace.freshness_not_loaded') ||
        'Viewport data has not loaded yet.'
      );
    }
    const diffMs = Math.max(0, currentTimeMs - lastMapDataLoadedAt);
    let age = 'just now';
    if (diffMs >= 3_600_000) {
      age = `${Math.round(diffMs / 3_600_000)}h ago`;
    } else if (diffMs >= 60_000) {
      age = `${Math.round(diffMs / 60_000)}m ago`;
    } else if (diffMs >= 10_000) {
      age = `${Math.round(diffMs / 1000)}s ago`;
    }
    return (
      (lastMapDataSource === 'cache'
        ? $t('admin.network.map.workspace.freshness_cache', { values: { age } })
        : $t('admin.network.map.workspace.freshness_live', { values: { age } })) ||
      (lastMapDataSource === 'cache'
        ? `Viewport data restored from cache ${age}.`
        : `Viewport data updated ${age}.`)
    );
  });
  const workspaceStatusNotes = $derived.by(() => {
    const notes: string[] = [mapDataFreshnessLabel];
    if (!workspaceCapabilities.canReadRouterInventory) {
      notes.push(
        $t('admin.network.map.workspace.router_overlay_unavailable') ||
          'Router overlay is unavailable for this role.',
      );
    } else if (!routerRows.length) {
      notes.push(
        $t('admin.network.map.workspace.router_overlay_empty') ||
          'Router overlay is enabled, but no router markers are available in this viewport.',
      );
    }
    if (!workspaceCapabilities.canReadCustomers && !workspaceCapabilities.canReadWorkOrders) {
      notes.push(
        $t('admin.network.map.workspace.customer_context_limited') ||
          'Customer and field context is limited for this role.',
      );
    }
    return notes;
  });
  const workspaceSubtitle = $derived.by(() => {
    const base =
      $t('admin.network.map.workspace.subtitle') ||
      'Visualize nodes, links, service zones, and operational context from the current viewport.';
    return `${base} ${workspaceStatusNotes[0] || ''}`.trim();
  });
  const workspaceSearchGroups = $derived.by(() =>
    buildNetworkMapOverviewSearchGroups({
      query: workspaceSearchQuery,
      quickMode,
      nodes: nodeRows,
      links: linkRows,
      zones: zoneRows,
      routers: routerRows,
      customerRows,
      serviceRows,
    }),
  );
  const workspaceSearchSummary = $derived.by(() => {
    const query = workspaceSearchQuery.trim();
    if (!query) {
      const total = nodeRows.length + linkRows.length + zoneRows.length + routerRows.length;
      return (
        $t('admin.network.map.search.summary_idle', {
          values: { count: total },
        }) || `${total} assets loaded in this workspace`
      );
    }
    const count = countNetworkMapSearchResults(workspaceSearchGroups);
    return (
      $t('admin.network.map.search.summary_results', {
        values: { count, groups: workspaceSearchGroups.length },
      }) || `${count} matching results across ${workspaceSearchGroups.length} sections`
    );
  });
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
  const compactMode = $derived($page.url.searchParams.get('compact') === '1');
  const fromInstallation = $derived($page.url.searchParams.get('from_installation') === '1');
  const sourceWorkOrderId = $derived($page.url.searchParams.get('work_order_id') || '');
  const sourceCustomerId = $derived($page.url.searchParams.get('customer_id') || '');
  const sourceLocationId = $derived($page.url.searchParams.get('location_id') || '');
  const assetMapTarget = $derived.by(() => parseNetworkAssetMapTarget($page.url.searchParams));
  const installationReturnUrl = $derived.by(() => {
    if (!fromInstallation) return '';
    const params = new URLSearchParams();
    if (sourceWorkOrderId) params.set('work_order_id', sourceWorkOrderId);
    return `${tenantPrefix}/admin/network/installations${params.toString() ? `?${params.toString()}` : ''}`;
  });

  onMount(() => {
    if (!canAccessNetworkMap($can)) {
      goto('/unauthorized');
      return;
    }
    if (typeof window !== 'undefined') {
      freshnessTimer = setInterval(() => {
        currentTimeMs = Date.now();
      }, 15_000);
    }
    workspaceState = applyNetworkMapWorkspaceDefaults(workspaceState, workspaceDefaults);
    void ensureChromeComponentsLoaded();
    if (workspaceCapabilities.canManageTopology) {
      applyQuickMode('all');
    } else if (workspaceCapabilities.canReadNetworkNoc && !workspaceCapabilities.canReadCustomers) {
      applyQuickMode('issues');
    } else if (workspaceCapabilities.canReadCustomers || workspaceCapabilities.canReadWorkOrders) {
      applyQuickMode('field');
    }
    ensureMaplibreCompatHelpers();
    void initMap();
  });

  onDestroy(() => {
    if (refreshDebounce) clearTimeout(refreshDebounce);
    if (freshnessTimer) clearInterval(freshnessTimer);
    activeDataAbortController?.abort();
    installationTargetMarker?.remove();
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
    syncWorkspaceHighlights();
  });

  $effect(() => {
    if (showCreateNodePanel || showLinkModal || showZoneModal || showDeleteConfirm) {
      void ensureDialogComponentsLoaded();
    }
  });

  async function ensureChromeComponentsLoaded() {
    if (OverviewComponent && FloatingControlsComponent) return;
    const modules = await loadNetworkMapChromeModules();
    OverviewComponent = modules.OverviewComponent;
    FloatingControlsComponent = modules.FloatingControlsComponent;
  }

  async function ensureDialogComponentsLoaded() {
    if (NodePanelComponent && LinkModalComponent && ZoneModalComponent && ConfirmDialogComponent) {
      return;
    }
    const modules = await loadNetworkMapDialogModules();
    NodePanelComponent = modules.NodePanelComponent;
    LinkModalComponent = modules.LinkModalComponent;
    ZoneModalComponent = modules.ZoneModalComponent;
    ConfirmDialogComponent = modules.ConfirmDialogComponent;
  }

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

  async function updateWorkspaceSelection(
    nextSelectedObject: ReturnType<typeof buildSelectedMapObject>,
  ) {
    const module = await getWorkspaceModule();
    workspaceState = module.selectNetworkMapObject(workspaceState, nextSelectedObject);
    if (workspaceDefaults.mode === 'investigate') {
      workspaceState = module.enterInvestigationMode(
        workspaceState,
        workspaceDefaults.investigationKind,
      );
    }
  }

  async function clearMapPopupSelection() {
    const module = await getWorkspaceModule();
    workspaceState = module.clearWorkspaceSelection(workspaceState);
  }

  function focusMapOnCoordinates(lng: number, lat: number, zoomFloor = 13) {
    if (!map || !Number.isFinite(lng) || !Number.isFinite(lat)) return;
    const currentZoom = Number.isFinite(map.getZoom()) ? map.getZoom() : zoomFloor;
    map.flyTo({
      center: [lng, lat],
      zoom: Math.max(currentZoom, zoomFloor),
      essential: true,
    });
  }

  function applyAssetMapTargetFocus() {
    if (assetFocusApplied) return;
    if (!assetMapTarget) return;

    const matchedRow = topologyAssetRows.find((row) => row.id === assetMapTarget.assetId);
    const lng = matchedRow?.longitude ?? assetMapTarget.longitude;
    const lat = matchedRow?.latitude ?? assetMapTarget.latitude;
    if (!Number.isFinite(lng) || !Number.isFinite(lat)) return;

    focusMapOnCoordinates(lng, lat, 15);
    assetFocusApplied = true;
  }

  function coordinateFromGeometry(geometry: Geometry | null | undefined): [number, number] | null {
    if (!geometry) return null;
    switch (geometry.type) {
      case 'Point':
        return geometry.coordinates as [number, number];
      case 'LineString': {
        const points = geometry.coordinates as Array<[number, number]>;
        if (!points.length) return null;
        return points[Math.floor(points.length / 2)] ?? points[0] ?? null;
      }
      case 'Polygon': {
        const ring = (geometry.coordinates?.[0] || []) as Array<[number, number]>;
        if (!ring.length) return null;
        const [sumLng, sumLat] = ring.reduce(
          (acc, [lng, lat]) => [acc[0] + lng, acc[1] + lat],
          [0, 0],
        );
        return [sumLng / ring.length, sumLat / ring.length];
      }
      case 'MultiPoint':
        return (geometry.coordinates?.[0] as [number, number]) ?? null;
      case 'MultiLineString': {
        const firstLine = (geometry.coordinates?.[0] || []) as Array<[number, number]>;
        if (!firstLine.length) return null;
        return firstLine[Math.floor(firstLine.length / 2)] ?? firstLine[0] ?? null;
      }
      case 'MultiPolygon': {
        const firstRing = (geometry.coordinates?.[0]?.[0] || []) as Array<[number, number]>;
        if (!firstRing.length) return null;
        const [sumLng, sumLat] = firstRing.reduce(
          (acc, [lng, lat]) => [acc[0] + lng, acc[1] + lat],
          [0, 0],
        );
        return [sumLng / firstRing.length, sumLat / firstRing.length];
      }
      case 'GeometryCollection':
        return coordinateFromGeometry(geometry.geometries?.[0]);
      default:
        return null;
    }
  }

  function escapePopupValue(input: unknown): string {
    return String(input ?? '-')
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#039;');
  }

  function popupToneForAssetStatus(status: string): 'ok' | 'warn' | 'muted' {
    if (status === 'installed' || status === 'available') return 'ok';
    if (status === 'reserved' || status === 'faulty') return 'warn';
    return 'muted';
  }

  function buildTopologyAssetPopupHtml(
    row: TopologyAssetRow,
    closeBtnId: string,
    connectBtnId: string,
  ) {
    const subtitle = [row.assetTypeLabel, row.locationLabel].filter(Boolean).join(' • ') || row.assetTypeLabel;
    const detailRows = [
      row.code ? { label: 'Code', value: row.code } : null,
      row.serialNumber ? { label: 'Serial', value: row.serialNumber } : null,
      row.customerName ? { label: 'Customer', value: row.customerName } : null,
      row.locationLabel ? { label: 'Location', value: row.locationLabel } : null,
      {
        label: 'Upstream',
        value: row.hasUpstreamRelation ? 'Linked to parent asset' : 'Not linked yet',
      },
      row.assetType === 'odp'
        ? {
            label: 'Customer Drop',
            value: row.hasCustomerRelation ? 'Linked to customer/service side' : 'Not linked yet',
          }
        : null,
      row.portCapacity != null ? { label: 'Port Capacity', value: String(row.portCapacity) } : null,
      row.portsUsed != null ? { label: 'Ports Used', value: String(row.portsUsed) } : null,
      row.portsAvailable != null
        ? { label: 'Ports Available', value: String(row.portsAvailable) }
        : null,
    ].filter(Boolean) as Array<{ label: string; value: string }>;

    return `
      <div class="nm-popup-card nm-popup-card-link">
        <div class="nm-popup-head">
          <div>
            <div class="nm-popup-kicker">FTTH Asset</div>
            <div class="nm-popup-title">${escapePopupValue(row.name)}</div>
            <div class="nm-popup-subtitle">${escapePopupValue(subtitle)}</div>
          </div>
          <span class="nm-popup-badge ${popupToneForAssetStatus(row.status)}">${escapePopupValue(row.status)}</span>
        </div>
        <div class="nm-popup-summary nm-popup-summary-link">
          <div class="nm-popup-summary-item">
            <div class="nm-popup-summary-label">Marker</div>
            <div class="nm-popup-summary-value">${escapePopupValue(row.markerLabel)}</div>
          </div>
          <div class="nm-popup-summary-item">
            <div class="nm-popup-summary-label">Type</div>
            <div class="nm-popup-summary-value">${escapePopupValue(row.assetTypeLabel)}</div>
          </div>
        </div>
        ${
          detailRows.length
            ? `<div class="nm-popup-grid">${detailRows
                .map(
                  (item) => `
                    <div>
                      <div class="nm-popup-label">${escapePopupValue(item.label)}</div>
                      <div class="nm-popup-value">${escapePopupValue(item.value)}</div>
                    </div>
                  `,
                )
                .join('')}</div>`
            : ''
        }
        <div class="nm-popup-actions nm-popup-actions-link">
          <button id="${connectBtnId}" class="nm-popup-btn primary" type="button">Connect</button>
          <button id="${closeBtnId}" class="nm-popup-btn nm-popup-btn-close" type="button">Close</button>
        </div>
      </div>
    `;
  }

  function applyQuickMode(nextMode: NetworkMapQuickMode) {
    quickMode = nextMode;
    if (nextMode === 'all') {
      nodesVisible = true;
      linksVisible = true;
      zonesVisible = true;
      routersVisible = true;
      customersVisible = true;
      topologyAssetsVisible = true;
      return;
    }

    if (nextMode === 'issues') {
      nodesVisible = true;
      linksVisible = true;
      zonesVisible = true;
      routersVisible = canReadRouterInventory;
      customersVisible = false;
      topologyAssetsVisible = true;
      return;
    }

    if (nextMode === 'customers') {
      nodesVisible = false;
      linksVisible = false;
      zonesVisible = false;
      routersVisible = false;
      customersVisible = true;
      topologyAssetsVisible = false;
      return;
    }

    if (nextMode === 'services') {
      nodesVisible = true;
      linksVisible = false;
      zonesVisible = false;
      routersVisible = false;
      customersVisible = true;
      topologyAssetsVisible = true;
      return;
    }

    if (nextMode === 'topology') {
      nodesVisible = true;
      linksVisible = true;
      zonesVisible = true;
      routersVisible = canReadRouterInventory;
      customersVisible = false;
      topologyAssetsVisible = true;
      return;
    }

    nodesVisible = true;
    linksVisible = false;
    zonesVisible = true;
    routersVisible = false;
    customersVisible = true;
    topologyAssetsVisible = true;
  }

  function handleWorkspaceSearchSelect(item: NetworkMapSearchResultItem) {
    workspaceSearchQuery = item.label;
    workspaceSearchOpen = false;

    if (item.kind === 'customer' || item.kind === 'service' || item.kind === 'node') {
      const row = nodeRows.find((candidate) => candidate.id === item.id);
      if (!row) return;
      focusMapOnCoordinates(row.lng, row.lat, 14);
      void updateWorkspaceSelection(
        buildSelectedMapObject({
          kind: item.kind,
          id: row.id,
          label: row.name,
          nodeType: row.node_type,
        }),
      );
      return;
    }

    if (item.kind === 'link') {
      const row = linkRows.find((candidate) => candidate.id === item.id);
      if (!row) return;
      const coord = coordinateFromGeometry(row.geometry);
      if (coord) focusMapOnCoordinates(coord[0], coord[1], 13);
      void updateWorkspaceSelection(
        buildSelectedMapObject({
          kind: 'link',
          id: row.id,
          label: row.name,
          linkType: row.link_type,
        }),
      );
      return;
    }

    if (item.kind === 'zone') {
      const row = zoneRows.find((candidate) => candidate.id === item.id);
      if (!row) return;
      const coord = coordinateFromGeometry(row.geometry);
      if (coord) focusMapOnCoordinates(coord[0], coord[1], 12);
      void updateWorkspaceSelection(
        buildSelectedMapObject({
          kind: 'zone',
          id: row.id,
          label: row.name,
          zoneType: row.zone_type,
        }),
      );
      return;
    }

    const router = routerRows.find((candidate) => candidate.id === item.id);
    if (!router) return;
    if (router.longitude != null && router.latitude != null) {
      focusMapOnCoordinates(Number(router.longitude), Number(router.latitude), 13);
    }
    void updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'router',
        id: router.id,
        label: router.identity || router.name,
      }),
    );
  }

  async function handleNodeLayerClick(e: any) {
    const clickedFeature = snapshotMapFeature(e.features?.[0]);
    if (!map || !clickedFeature || !maplibre) return;
    const props = clickedFeature.properties || {};
    const nodeId = String(props.id || '');
    await updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'node',
        id: nodeId,
        label: String(props.name || props.label || nodeId),
        nodeType: props.node_type || props.nodeType || undefined,
      }),
    );
    if (linkPickMode) {
      handleLinkPickNode(nodeId);
      return;
    }
    const { openNodePopup } = await loadNetworkMapPopupModule();
    openNodePopup({
      map,
      maplibre,
      feature: clickedFeature.feature as any,
      nodeRows,
      routerRows,
      activePopup: activeNodePopup,
      setActivePopup: (popup) => (activeNodePopup = popup),
      onClose: clearMapPopupSelection,
      onOpenCustomer: (customerId) => void goto(`${tenantPrefix}/admin/customers/${customerId}`),
      onOpenService: (customerId, serviceId) =>
        void goto(
          `${tenantPrefix}/admin/customers/${customerId}?tab=subscriptions&service_id=${encodeURIComponent(serviceId)}`,
        ),
      onConnect: startConnectFromNode,
      onEdit: openEditNodeModal,
      onOpenRouter: (routerId) => void goto(`${tenantPrefix}/admin/network/routers/${routerId}`),
    });
  }

  async function handleLinkLayerClick(e: any) {
    const clickedFeature = snapshotMapFeature(e.features?.[0]);
    if (!map || !clickedFeature || !maplibre || linkPickMode) return;
    const props = clickedFeature.properties || {};
    const linkId = String(props.id || '');
    await updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'link',
        id: linkId,
        label: String(props.name || props.label || linkId),
        linkType: props.link_type || props.linkType || undefined,
      }),
    );
    const { openLinkPopup } = await loadNetworkMapPopupModule();
    openLinkPopup({
      map,
      maplibre,
      feature: clickedFeature.feature as any,
      lngLat: e.lngLat,
      linkRows,
      onClose: clearMapPopupSelection,
      onEdit: openEditLinkModal,
      onDelete: (linkId, linkName) => openDeleteConfirm('link', linkId, linkName),
    });
  }

  async function handleRouterLayerClick(e: any) {
    const clickedFeature = snapshotMapFeature(e.features?.[0]);
    if (!map || !clickedFeature || !maplibre) return;
    const props = clickedFeature.properties || {};
    const routerId = String(props.id || '');
    await updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'router',
        id: routerId,
        label: String(props.name || props.identity || routerId),
      }),
    );
    const { openRouterPopup } = await loadNetworkMapPopupModule();
    openRouterPopup({
      map,
      maplibre,
      feature: clickedFeature.feature as any,
      activePopup: activeNodePopup,
      setActivePopup: (popup) => (activeNodePopup = popup),
      onClose: clearMapPopupSelection,
      onOpenRouter: (routerId) => void goto(`${tenantPrefix}/admin/network/routers/${routerId}`),
    });
  }

  async function handleTopologyAssetLayerClick(e: any) {
    const clickedFeature = snapshotMapFeature(e.features?.[0]);
    if (!map || !clickedFeature || !maplibre) return;
    const assetId = String(clickedFeature.properties?.id || '');
    const row = topologyAssetRows.find((candidate) => candidate.id === assetId);
    const coords = coordinateFromGeometry(clickedFeature.feature.geometry as Geometry);
    if (!row || !coords) return;
    if (linkPickMode) {
      const nodeId = await ensureTopologyAssetNodeId(assetId);
      if (!nodeId) {
        toast.error('FTTH asset node belum tersinkron ke topology map.');
        return;
      }
      handleLinkPickNode(nodeId);
      return;
    }
    const mapInstance = map;
    const { bindPopupNavigationDismiss } = await loadNetworkMapPopupModule();

    activeNodePopup?.remove();
    const closeBtnId = `nm-topology-asset-close-${Math.random().toString(36).slice(2, 10)}`;
    const connectBtnId = `nm-topology-asset-connect-${Math.random().toString(36).slice(2, 10)}`;
    const popup = new maplibre.Popup({
      closeButton: false,
      closeOnClick: true,
      anchor: 'bottom',
      offset: 14,
    })
      .setLngLat(coords)
      .setHTML(buildTopologyAssetPopupHtml(row, closeBtnId, connectBtnId));
    let cleanupNavigationDismiss: (() => void) | null = null;

    popup.on('open', () => {
      requestAnimationFrame(() => {
        const popupElement =
          typeof (popup as any).getElement === 'function'
            ? ((popup as any).getElement() as HTMLElement)
            : null;
        popupElement?.classList.add('nm-popup-link-shell');
      });
      cleanupNavigationDismiss = bindPopupNavigationDismiss({
        map: mapInstance,
        popup,
      });
      const closeBtn = document.getElementById(closeBtnId) as HTMLButtonElement | null;
      const connectBtn = document.getElementById(connectBtnId) as HTMLButtonElement | null;
      connectBtn?.addEventListener('click', () => {
        popup.remove();
        void startConnectFromTopologyAsset(assetId);
      });
      closeBtn?.addEventListener('click', () => popup.remove());
    });
    popup.on('close', () => {
      cleanupNavigationDismiss?.();
      cleanupNavigationDismiss = null;
      activeNodePopup = null;
    });

    activeNodePopup = popup;
    popup.addTo(mapInstance);
  }

  async function refreshTopologyAssets(force = false) {
    const now = Date.now();
    const isStale = now - lastTopologyAssetsLoadedAt >= topologyAssetsCacheTtlMs;
    if (!force && !topologyAssetsVisible && topologyAssetRows.length) {
      return;
    }
    if (!force && !isStale && topologyAssetRows.length) {
      replaceTopologyAssetOverlay(topologyAssetsToFeatureCollection(topologyAssetRows));
      syncTopologyAssetLinkOverlay();
      syncLayerVisibility();
      return;
    }
    if (refreshingTopologyAssets) return;

    refreshingTopologyAssets = true;
    try {
      const response = await networkAssets.list({
        page: 1,
        per_page: 500,
      });
      topologyAssetItems = (response.data || []) as NetworkAssetListItem[];
      topologyAssetRows = buildTopologyAssetRows(topologyAssetItems);
      lastTopologyAssetsLoadedAt = Date.now();
      replaceTopologyAssetOverlay(topologyAssetsToFeatureCollection(topologyAssetRows));
      syncTopologyAssetLinkOverlay();
      syncLayerVisibility();
      applyAssetMapTargetFocus();
    } catch (error) {
      console.error(error);
    } finally {
      refreshingTopologyAssets = false;
    }
  }

  async function getInteractionModule() {
    if (interactionModule) return interactionModule;
    interactionModule = await loadNetworkMapInteractionModule();
    return interactionModule;
  }

  async function getWorkspaceModule() {
    if (workspaceModule) return workspaceModule;
    workspaceModule = await loadNetworkMapWorkspaceModule();
    return workspaceModule;
  }

  async function initMap() {
    try {
      const [loadedMaplibre, loadedInteractionModule] = await Promise.all([
        import('maplibre-gl'),
        getInteractionModule(),
      ]);
      maplibre = loadedMaplibre;
      if (!mapEl || !maplibre) return;

      map = new maplibre.Map({
        container: mapEl,
        style: loadedInteractionModule.buildBaseMapStyle({
          hasHiResSatellite,
          mapTilerKey,
          standardMaxZoom,
          satelliteMaxZoom,
        }),
        center: [106.8456, -6.2088],
        zoom: 10,
        maxZoom: standardMaxZoom,
        minZoom: 3,
      });

      map.on('load', async () => {
        if (!map) return;
        loadedInteractionModule.ensureNodeTypeIconsRegistered(map);
        loadedInteractionModule.registerMapSourcesAndLayers(map);

        loadedInteractionModule.registerPrimaryLayerClicks({
          map,
          onNodeClick: handleNodeLayerClick,
          onRouterClick: handleRouterLayerClick,
          onTopologyAssetClick: handleTopologyAssetLayerClick,
          onLinkClick: handleLinkLayerClick,
          onCustomerClusterClick: async (e) => {
            if (!map || !maplibre || !e.features?.[0]) return;
            try {
              await loadedInteractionModule.expandCustomerCluster({
                map,
                feature: e.features[0],
                sourceId: SOURCE_CUSTOMERS,
              });
            } catch (error) {
              console.error(error);
            }
          },
        });

        map.on('click', (e) => {
          if (!map) return;
          const result = loadedInteractionModule.handleCanvasMapClick({
            map,
            event: e,
            linkPickMode,
            linkPickDrawMode,
            linkForm,
            linkSnapToNodeEnabled,
            nodeRows,
            nodePickMode,
            onAddLinkPathPoint: (point) => {
              linkPathBendPoints = [...linkPathBendPoints, point];
              refreshLinkGeometryDraft();
              syncLinkDraftPreview();
            },
            onApplyPickedNodeCoordinates: applyPickedNodeCoordinates,
          });
          if (result.handled) return;
        });

        loadedInteractionModule.registerInteractiveLayerHover(map);

        map.on('moveend', scheduleRefresh);
        mapReady = true;
        syncLayerVisibility();
        syncLinkDraftPreview();
        await refreshMapData();
        if (!installationTargetResolved) {
          installationTargetResolved = true;
          try {
            const resolved = await resolveInstallationTargetMarker({
              map,
              maplibre,
              fromInstallation,
              sourceCustomerId,
              sourceLocationId,
              compactMode,
              didInitialFitToMarkers,
              existingMarker: installationTargetMarker,
              loadCustomerLocations: (customerId) => customers.locations.list(customerId),
            });
            if (resolved) {
              installationTargetMarker = resolved.marker;
              installationTargetCoord = resolved.coord;
            }
          } catch (error) {
            console.error(error);
          }
        }
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

  async function syncTopologyAssets(manual = false) {
    if (syncingAssetNodes) return false;
    syncingAssetNodes = true;
    try {
      const result = await syncTopologyAssetsIfNeeded({
        canManageTopology,
        syncingAssetNodes: false,
        manual,
        lastAssetSyncAt,
        assetSyncTtlMs,
      });
      lastAssetSyncAt = result.lastSyncedAt;
      return result.changed;
    } finally {
      syncingAssetNodes = false;
    }
  }

  function queueBackgroundTopologySync() {
    if (backgroundAssetSyncPromise || syncingAssetNodes) return;
    backgroundAssetSyncPromise = syncTopologyAssets(false)
      .then(async (changed) => {
        if (!changed) return false;
        invalidateMapDataCache();
        await refreshMapData(true, { skipAutoSync: true });
        return true;
      })
      .finally(() => {
        backgroundAssetSyncPromise = null;
      });
  }

  async function refreshMapData(force = false, options?: { skipAutoSync?: boolean }) {
    if (map && !mapReady) return;
    const requestId = ++lastRequestId;
    const viewportBbox = currentBboxString();
    if (!viewportBbox) return;

    refreshing = true;

    try {
      let shouldBypassCache = force;
      if (!options?.skipAutoSync) {
        const syncStrategy = getTopologySyncStrategy({
          canManageTopology,
          syncingAssetNodes,
          manual: force,
          lastAssetSyncAt,
          assetSyncTtlMs,
        });
        if (syncStrategy.shouldBlockRefresh) {
          if (await syncTopologyAssets(force)) {
            shouldBypassCache = true;
            invalidateMapDataCache();
          }
        } else if (syncStrategy.shouldSync) {
          queueBackgroundTopologySync();
        }
      }

      const hasActiveFilters = !!(q.trim() || status || kind);
      const bbox = resolveNetworkMapFetchBbox({
        viewportBbox,
        initialExtentLoaded,
        hasActiveFilters,
      });
      const isInitialExtentRequest = bbox !== viewportBbox;

      const params = {
        q: q.trim() || undefined,
        status: status || undefined,
        kind: kind || undefined,
        bbox,
        page: 1,
        per_page: 1000,
      };

      const zoomSig = map ? map.getZoom().toFixed(2) : '0.00';
      const cacheKey = buildMapDataCacheKey(params, zoomSig);
      const cached = shouldBypassCache
        ? undefined
        : getCachedMapData(dataCache, cacheKey, dataCacheTtlMs);
      if (cached) {
        if (requestId !== lastRequestId) return;
        lastMapDataLoadedAt = cached.at;
        lastMapDataSource = 'cache';
        applyCachedMapData({
          cached,
          setRows: (rows) => {
            nodeRows = rows.nodeRows;
            linkRows = rows.linkRows;
            zoneRows = rows.zoneRows;
            routerRows = rows.routerRows;
            customerRows = rows.customerRows;
            serviceRows = rows.serviceRows;
            nodeCount = rows.nodeCount;
            linkCount = rows.linkCount;
            zoneCount = rows.zoneCount;
          },
          setSourceData,
          sourceIds: {
            nodes: SOURCE_NODES,
            customers: SOURCE_CUSTOMERS,
            links: SOURCE_LINKS,
            zones: SOURCE_ZONES,
            routers: SOURCE_ROUTERS,
          },
          fitToMarkers: fitMapToAllMarkersOnFirstLoad,
        });
        await refreshTopologyAssets(force);
        if (isInitialExtentRequest) initialExtentLoaded = true;
        return;
      }

      activeDataAbortController?.abort();
      const abortController = new AbortController();
      activeDataAbortController = abortController;

      const result = await fetchNetworkMapData(params, abortController.signal, {
        includeRouters: shouldFetchRouterOverlay({
          canReadRouterInventory,
          routersVisible,
        }),
      });

      // Drop stale responses when user moves map quickly.
      if (requestId !== lastRequestId) return;
      if (abortController.signal.aborted) return;

      setCachedMapData(
        dataCache,
        cacheKey,
        {
          nodes: result.nodesRes,
          links: result.linksRes,
          zones: result.zonesRes,
          routers: result.routersRes,
        },
        dataCacheMaxEntries,
      );
      lastMapDataLoadedAt = Date.now();
      lastMapDataSource = 'live';

      applyFetchedMapData({
        result,
        setRows: (nextRows) => {
          nodeRows = nextRows.nodeRows;
          linkRows = nextRows.linkRows;
          zoneRows = nextRows.zoneRows;
          routerRows = nextRows.routerRows;
          customerRows = nextRows.customerRows;
          serviceRows = nextRows.serviceRows;
          nodeCount = nextRows.nodeCount;
          linkCount = nextRows.linkCount;
          zoneCount = nextRows.zoneCount;
        },
        setSourceData,
        sourceIds: {
          nodes: SOURCE_NODES,
          customers: SOURCE_CUSTOMERS,
          links: SOURCE_LINKS,
          zones: SOURCE_ZONES,
          routers: SOURCE_ROUTERS,
        },
        fitToMarkers: fitMapToAllMarkersOnFirstLoad,
      });
      await refreshTopologyAssets(force);
      syncTopologyAssetLinkOverlay();
      if (isInitialExtentRequest) initialExtentLoaded = true;
    } catch (e: any) {
      if ((e?.message || '').includes('Request canceled')) return;
      console.error(e);
    } finally {
      if (requestId === lastRequestId) activeDataAbortController = null;
      refreshing = false;
    }
  }

  function fitMapToAllMarkersOnFirstLoad(nodes: NMNode[], routers: NMRouter[]) {
    if (!map || !maplibre || !interactionModule) return;
    const didFit = interactionModule.fitMapToMarkers({
      map,
      maplibre,
      didInitialFitToMarkers,
      nodes,
      routers,
      topologyAssets: topologyAssetRows,
      installationTargetCoord,
    });
    if (didFit) didInitialFitToMarkers = true;
  }

  function setSourceData(sourceId: string, data: GeoJSON.FeatureCollection) {
    if (!map) return;
    if (!map.getSource(sourceId)) return;
    const source = map.getSource(sourceId) as import('maplibre-gl').GeoJSONSource | undefined;
    source?.setData(data as any);
  }

  function syncTopologyAssetLinkOverlay() {
    setSourceData(
      SOURCE_TOPOLOGY_ASSET_LINKS,
      buildTopologyAssetAutoLinkFeatureCollection({
        assets: topologyAssetItems,
        topologyRows: topologyAssetRows,
        customerNodes: nodeRows,
        nodeRows,
        linkRows,
      }),
    );
  }

  function replaceTopologyAssetOverlay(data: GeoJSON.FeatureCollection) {
    if (!map) return;
    for (const layerId of [
      'nm-topology-assets-label',
      'nm-topology-assets-icon',
      'nm-topology-assets-circle',
      'nm-topology-assets-halo',
    ]) {
      if (map.getLayer(layerId)) {
        map.removeLayer(layerId);
      }
    }

    if (map.getSource(SOURCE_TOPOLOGY_ASSETS)) {
      map.removeSource(SOURCE_TOPOLOGY_ASSETS);
    }

    map.addSource(SOURCE_TOPOLOGY_ASSETS, {
      type: 'geojson',
      data: JSON.parse(JSON.stringify(data)),
    });

    map.addLayer({
      id: 'nm-topology-assets-halo',
      type: 'circle',
      source: SOURCE_TOPOLOGY_ASSETS,
      paint: {
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 8, 11, 10.5, 14, 13],
        'circle-color': ['coalesce', ['get', 'marker_color'], '#64748b'],
        'circle-opacity': 0.16,
        'circle-blur': 0.08,
        'circle-stroke-width': 0,
      },
    });

    map.addLayer({
      id: 'nm-topology-assets-circle',
      type: 'circle',
      source: SOURCE_TOPOLOGY_ASSETS,
      paint: {
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 6.2, 11, 8, 14, 9.6],
        'circle-color': ['coalesce', ['get', 'marker_color'], '#64748b'],
        'circle-opacity': 0.38,
        'circle-stroke-width': 2,
        'circle-stroke-color': '#e2e8f0',
      },
    });

    map.addLayer({
      id: 'nm-topology-assets-icon',
      type: 'symbol',
      source: SOURCE_TOPOLOGY_ASSETS,
      layout: {
        'icon-image': [
          'match',
          ['get', 'asset_type'],
          'olt',
          'nm-node-icon-olt',
          'odc',
          'nm-node-icon-odc',
          'odp',
          'nm-node-icon-odp',
          'fat',
          'nm-node-icon-odp',
          'nap',
          'nm-node-icon-odp',
          'switch',
          'nm-node-icon-switch',
          'nm-node-icon-router',
        ],
        'icon-size': ['interpolate', ['linear'], ['zoom'], 8, 0.78, 11, 0.96, 14, 1.14],
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
      },
    });
  }

  function syncWorkspaceHighlights() {
    if (!map || !mapReady || !interactionModule) return;

    const selectedObject =
      workspaceState.investigationState?.selectedObject || workspaceState.selectedObject;

    const { pointData, lineData, zoneData } = interactionModule.buildSelectionFeatureCollections({
      selectedObject,
      nodeRows,
      linkRows,
      zoneRows,
      routerRows,
    });

    setSourceData(SOURCE_SELECTION_POINTS, pointData);
    setSourceData(SOURCE_SELECTION_LINES, lineData);
    setSourceData(SOURCE_SELECTION_ZONES, zoneData);
  }

  function setLayerVisibility(layerId: string, visible: boolean) {
    if (!map || !map.getLayer(layerId)) return;
    map.setLayoutProperty(layerId, 'visibility', visible ? 'visible' : 'none');
  }

  function syncLayerVisibility() {
    if (!map || !mapReady) return;
    setLayerVisibility('nm-zones-fill', zonesVisible);
    setLayerVisibility('nm-zones-outline', zonesVisible);
    setLayerVisibility('nm-links-line', linksVisible);
    setLayerVisibility('nm-links-line-dashed', linksVisible);
    setLayerVisibility('nm-topology-asset-links-parent', linksVisible && topologyAssetsVisible);
    setLayerVisibility('nm-topology-asset-links-customer', linksVisible && topologyAssetsVisible);
    setLayerVisibility('nm-nodes-circle', nodesVisible);
    setLayerVisibility('nm-nodes-icons', nodesVisible);
    setLayerVisibility('nm-routers-circle', routersVisible);
    setLayerVisibility('nm-routers-icon', routersVisible);
    setLayerVisibility('nm-topology-assets-halo', topologyAssetsVisible);
    setLayerVisibility('nm-topology-assets-circle', topologyAssetsVisible);
    setLayerVisibility('nm-topology-assets-icon', topologyAssetsVisible);
    setLayerVisibility('nm-customers-cluster-circle', customersVisible);
    setLayerVisibility('nm-customers-cluster-count', customersVisible);
    setLayerVisibility('nm-customers-point', customersVisible);
  }

  function setNodesVisible(checked: boolean) {
    nodesVisible = checked;
    syncLayerVisibility();
  }

  function setLinksVisible(checked: boolean) {
    linksVisible = checked;
    syncLayerVisibility();
  }

  function setZonesVisible(checked: boolean) {
    zonesVisible = checked;
    syncLayerVisibility();
  }

  function setCustomersVisible(checked: boolean) {
    customersVisible = checked;
    syncLayerVisibility();
  }

  function setTopologyAssetsVisible(checked: boolean) {
    topologyAssetsVisible = checked;
    syncLayerVisibility();
    if (checked) {
      void refreshTopologyAssets();
    }
  }

  function setRoutersVisible(checked: boolean) {
    routersVisible = checked;
    syncLayerVisibility();
    if (checked && canReadRouterInventory && routerRows.length === 0) {
      void refreshMapData(true);
    }
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

  function defaultZoneGeometry() {
    return buildDefaultZoneGeometry(map);
  }

  function refreshLinkGeometryDraft() {
    linkForm.geometryText = buildLinkGeometryDraftText({
      linkPickDrawMode,
      nodeRows,
      linkForm,
      linkPathBendPoints,
    });
  }

  function syncLinkDraftPreview() {
    const { lineFc, pointsFc } = buildLinkDraftPreviewCollections({
      linkPickMode,
      linkPickDrawMode,
      nodeRows,
      linkForm,
      linkPathBendPoints,
    });

    setSourceData(SOURCE_LINK_DRAFT, lineFc);
    setSourceData(SOURCE_LINK_DRAFT_POINTS, pointsFc);
    setLayerVisibility('nm-link-draft-line', linkPickMode);
    setLayerVisibility('nm-link-draft-points', linkPickMode);
  }

  function stopNodePickMode(removeMarker = false) {
    nodePickMode = false;
    draftNodeMarker = clearDraftNodeMarker(draftNodeMarker, removeMarker);
  }

  function applyPickedNodeCoordinates(lng: number, lat: number) {
    nodeForm.lat = lat.toFixed(6);
    nodeForm.lng = lng.toFixed(6);
    if (!maplibre || !map) return;
    draftNodeMarker = applyPickedNodeMarker({
      map,
      maplibre,
      marker: draftNodeMarker,
      lng,
      lat,
      onDrag: (nextLng, nextLat) => {
        nodeForm.lat = nextLat.toFixed(6);
        nodeForm.lng = nextLng.toFixed(6);
      },
    });
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
    if (isSystemManagedNode(row)) {
      toast.info(
        `Node ini tersinkron dari ${systemManagedNodeSourceLabel(row) || 'asset map'}. Ubah dari sumbernya.`,
      );
      return;
    }
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
    savingNode = true;
    try {
      const ok = await submitNodeCrud({
        editingNodeId,
        nodeForm,
      });
      if (!ok) return;
      closeNodeModal();
      invalidateMapDataCache();
      await refreshMapData(true);
    } finally {
      savingNode = false;
    }
  }

  function openCreateLinkModal() {
    activeAssetConnectSourceId = null;
    editingLinkId = null;
    linkPickMode = false;
    linkPickStep = 'from';
    linkPickDrawMode = 'quick';
    linkPathBendPoints = [];
    linkForm = createLinkForm(nodeRows);
    showLinkModal = true;
    syncLinkDraftPreview();
  }

  function openEditLinkModal(row: NMLink) {
    activeAssetConnectSourceId = null;
    linkPickMode = false;
    linkPickStep = 'from';
    linkPickDrawMode = 'quick';
    linkPathBendPoints = [];
    editingLinkId = row.id;
    linkForm = createEditLinkForm(row, nodeRows);
    showLinkModal = true;
    syncLinkDraftPreview();
  }

  function toggleLinkPickMode() {
    const next = buildToggleLinkPickResult({
      currentEnabled: linkPickMode,
      drawMode: linkPickDrawMode,
      nodeRows,
    });
    linkPickMode = next.linkPickMode;
    linkPickStep = next.linkPickStep;
    linkPathBendPoints = next.linkPathBendPoints;
    if (next.resetFromNodeId || next.resetToNodeId) {
      linkForm = {
        ...linkForm,
        from_node_id: '',
        to_node_id: '',
        geometryText: next.geometryText,
      };
    }
    if (next.toastMessage) toast.info(next.toastMessage);
    syncLinkDraftPreview();
  }

  function closeLinkModal() {
    showLinkModal = false;
    activeAssetConnectSourceId = null;
    linkPickMode = false;
    linkPickStep = 'from';
    linkPickDrawMode = 'quick';
    linkPathBendPoints = [];
    syncLinkDraftPreview();
  }

  function setLinkPickDrawMode(mode: LinkPickDrawMode) {
    const next = buildSetLinkDrawModeResult({
      mode,
      linkPickMode,
      nodeRows,
    });
    linkPickDrawMode = next.linkPickDrawMode;
    linkPathBendPoints = next.linkPathBendPoints;
    linkPickStep = next.linkPickStep;
    linkForm = {
      ...linkForm,
      from_node_id: '',
      to_node_id: '',
      geometryText: next.geometryText,
    };
    if (next.toastMessage) toast.info(next.toastMessage);
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

  function cancelLinkPicking() {
    if (!linkPickMode) return;
    activeAssetConnectSourceId = null;
    linkPickMode = false;
    linkPickStep = 'from';
    linkPathBendPoints = [];
    syncLinkDraftPreview();
    toast.info('Link drawing canceled.');
  }

  function handleLinkPickNode(nodeId: string) {
    const result = buildHandlePickedLinkNodeResult({
      nodeId,
      linkPickMode,
      linkPickStep,
      linkPickDrawMode,
      linkRows,
      nodeRows,
      linkForm,
      editingLinkId,
    });
    if (result.kind === 'noop') return;
    if (result.kind === 'error') {
      toast.error(result.toastMessage);
      return;
    }

    linkForm = result.linkForm;
    if (result.kind === 'picked-from') {
      linkPathBendPoints = result.linkPathBendPoints;
      linkPickStep = result.linkPickStep;
      if (linkPickDrawMode === 'path') {
        refreshLinkGeometryDraft();
      }
      toast.info(result.toastMessage);
      syncLinkDraftPreview();
      return;
    }

    linkPickMode = result.linkPickMode;
    linkPickStep = result.linkPickStep;
    showLinkModal = result.showLinkModal;
    if (linkPickDrawMode === 'quick') {
      useLinkFromNodePoints();
    } else {
      refreshLinkGeometryDraft();
    }
    syncLinkDraftPreview();
    toast.success(result.toastMessage);
  }

  async function startConnectFromNode(nodeId: string) {
    activeNodePopup?.remove();
    activeAssetConnectSourceId = null;
    const next = buildConnectFromNodeResult(nodeId, nodeRows);
    const nodeRow = nodeRows.find((row) => row.id === nodeId);
    await updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'node',
        id: nodeId,
        label: nodeRow?.name || nodeId,
        nodeType: nodeRow?.node_type || undefined,
      }),
    );
    editingLinkId = next.editingLinkId;
    showLinkModal = next.showLinkModal;
    linkPickDrawMode = next.linkPickDrawMode;
    linkPickMode = next.linkPickMode;
    linkPickStep = next.linkPickStep;
    linkPathBendPoints = next.linkPathBendPoints;
    linkForm = next.linkForm;
    refreshLinkGeometryDraft();
    syncLinkDraftPreview();
    toast.info(next.toastMessage);
  }

  async function ensureTopologyAssetNodeId(assetId: string) {
    let nodeId = findTopologyAssetNodeId(nodeRows, assetId);
    if (nodeId) return nodeId;
    await syncTopologyAssets(true);
    invalidateMapDataCache();
    await refreshMapData(true, { skipAutoSync: true });
    nodeId = findTopologyAssetNodeId(nodeRows, assetId);
    return nodeId;
  }

  async function startConnectFromTopologyAsset(assetId: string) {
    activeNodePopup?.remove();
    const nodeId = await ensureTopologyAssetNodeId(assetId);
    if (!nodeId) {
      toast.error('FTTH asset node belum tersedia di topology map.');
      return;
    }
    activeAssetConnectSourceId = assetId;
    const next = buildConnectFromNodeResult(nodeId, nodeRows);
    const assetRow = topologyAssetRows.find((row) => row.id === assetId);
    await updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'node',
        id: nodeId,
        label: assetRow?.name || assetId,
        nodeType: assetRow?.assetType || undefined,
      }),
    );
    editingLinkId = next.editingLinkId;
    showLinkModal = next.showLinkModal;
    linkPickDrawMode = next.linkPickDrawMode;
    linkPickMode = next.linkPickMode;
    linkPickStep = next.linkPickStep;
    linkPathBendPoints = next.linkPathBendPoints;
    linkForm = next.linkForm;
    refreshLinkGeometryDraft();
    syncLinkDraftPreview();
    toast.info(next.toastMessage);
  }

  async function syncAssetRelationsFromSavedLink(sourceAssetId: string, targetNodeId: string) {
    const sourceAsset = topologyAssetItems.find((item) => item.id === sourceAssetId);
    const targetNode = nodeRows.find((row) => row.id === targetNodeId);
    if (!sourceAsset || !targetNode) return;

    const operations = buildTopologyAssetConnectionOperations({
      sourceAsset,
      targetNode,
    });
    for (const operation of operations) {
      const currentAsset = topologyAssetItems.find((item) => item.id === operation.assetId);
      if (!currentAsset) continue;
      const currentDraft = buildTopologyAssetConnectDraft(currentAsset);
      const nextParentAssetId =
        operation.parentAssetId === undefined ? currentDraft.parentAssetId : String(operation.parentAssetId || '').trim();
      const nextCustomerId =
        operation.customerId === undefined ? currentDraft.customerId : String(operation.customerId || '').trim();
      const nextLocationId =
        operation.locationId === undefined ? currentDraft.locationId : String(operation.locationId || '').trim();

      if (nextParentAssetId !== currentDraft.parentAssetId) {
        await networkAssets.linkParentAsset(operation.assetId, nextParentAssetId || null);
      }
      if (nextCustomerId !== currentDraft.customerId) {
        await networkAssets.assignCustomer(operation.assetId, nextCustomerId || null);
      }
      if (nextLocationId !== currentDraft.locationId) {
        await networkAssets.assignLocation(operation.assetId, nextLocationId || null);
      }
    }
  }

  function useLinkFromNodePoints() {
    linkForm.geometryText = buildStraightLinkGeometryText(
      nodeRows,
      linkForm.from_node_id,
      linkForm.to_node_id,
    );
    syncLinkDraftPreview();
  }

  async function submitLink() {
    savingLink = true;
    try {
      linkForm.geometryText = resolveLinkGeometryTextForSubmit(linkForm, nodeRows);
      const ok = await submitLinkCrud({
        editingLinkId,
        linkForm,
        linkFieldConfig,
        hasExistingLinkBetweenNodes: (fromNodeId, toNodeId, excludeLinkId) =>
          hasExistingLinkBetweenNodes(linkRows, fromNodeId, toNodeId, excludeLinkId),
      });
      if (!ok) return;
      if (activeAssetConnectSourceId) {
        await syncAssetRelationsFromSavedLink(activeAssetConnectSourceId, linkForm.to_node_id);
      }
      emitInstallationRefreshSignal({
        fromInstallation,
        sourceWorkOrderId,
      });
      emitWorkOrderUpdatedToParent({
        fromInstallation,
        sourceWorkOrderId,
      });
      closeLinkModal();
      invalidateMapDataCache();
      await refreshMapData(true);
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
    const draft = buildZoneDraftForm(row, defaultZoneGeometry());
    zoneForm = {
      ...draft,
      geometryText: prettyGeometry(draft.geometry as GeoJSON.Geometry),
    };
    showZoneModal = true;
  }

  async function submitZone() {
    savingZone = true;
    try {
      const ok = await submitZoneCrud({
        editingZoneId,
        zoneForm,
      });
      if (!ok) return;
      showZoneModal = false;
      invalidateMapDataCache();
      await refreshMapData(true);
    } finally {
      savingZone = false;
    }
  }

  async function removeNode(id: string) {
    deletingId = id;
    try {
      const ok = await removeCrud({ type: 'node', id });
      if (!ok) return;
      invalidateMapDataCache();
      await refreshMapData(true);
    } finally {
      deletingId = null;
    }
  }

  async function removeLink(id: string) {
    deletingId = id;
    try {
      const ok = await removeCrud({ type: 'link', id });
      if (!ok) return;
      invalidateMapDataCache();
      await refreshMapData(true);
    } finally {
      deletingId = null;
    }
  }

  async function removeZone(id: string) {
    deletingId = id;
    try {
      const ok = await removeCrud({ type: 'zone', id });
      if (!ok) return;
      invalidateMapDataCache();
      await refreshMapData(true);
    } finally {
      deletingId = null;
    }
  }

  function openDeleteConfirm(targetType: 'node' | 'link' | 'zone', id: string, name?: string) {
    deleteTargetType = targetType;
    deleteTargetId = id;
    const copy = buildDeleteConfirmCopy(targetType, name);
    deleteConfirmTitle = copy.title;
    deleteConfirmMessage = copy.message;
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
    } else {
      await removeZone(id);
    }
    deleteTargetType = null;
    deleteTargetId = '';
  }
</script>

<div class="page-content fade-in" class:compact-mode={compactMode}>
  {#if OverviewComponent}
    <OverviewComponent
      {compactMode}
      {fromInstallation}
      {installationReturnUrl}
      {tenantPrefix}
      {canManageTopology}
      {syncingAssetNodes}
      {refreshing}
      {loading}
      title={$t('admin.network.map.title') || 'Network Topology Map'}
      subtitle={workspaceSubtitle}
      labels={{
        backToInstallation: $t('admin.network.map.back_to_installation') || 'Back to Installation',
        backToNoc: $t('admin.network.map.back_to_noc') || 'Back to NOC',
        syncing: 'Syncing...',
        sync: 'Sync',
      }}
      onSyncAssets={async () => {
        if (await syncTopologyAssets(true)) {
          invalidateMapDataCache();
        }
        await refreshMapData(true);
      }}
    />
  {/if}

  <MapCanvasShell
    bind:mapEl
    bind:viewMode
    showSearch={false}
    showViewSwitch={false}
    {loading}
    {mapUnavailable}
    {mapErrorMessage}
    mapUnavailableTitle="Map preview unavailable on this device"
    mapUnavailableSubtitle="WebGL context failed. Data is still loaded and counts are visible."
    height={compactMode ? 'min(82vh, 820px)' : 'calc(100vh - 150px)'}
  >
    <svelte:fragment slot="overlay">
      <div class="map-workspace-search">
        <button
          class="map-workspace-search-toggle"
          class:active={workspaceSearchOpen}
          type="button"
          aria-label="Search map assets"
          title="Search map assets"
          onclick={() => (workspaceSearchOpen = !workspaceSearchOpen)}
        >
          <Icon name="search" size={18} />
        </button>
        {#if workspaceSearchOpen}
          <div class="map-workspace-search-panel">
            <NetworkMapSearchBar
              query={workspaceSearchQuery}
              groups={workspaceSearchGroups}
              summary=""
              placeholder={$t('admin.network.map.search.placeholder') ||
                'Cari customer, node, link, zone, atau router...'}
              emptyTitle={$t('admin.network.map.search.empty_title') || 'No matching results'}
              emptyHint={$t('admin.network.map.search.empty_hint') ||
                'Coba kata kunci lain.'}
              onQueryChange={(value: string) => (workspaceSearchQuery = value)}
              onSelect={handleWorkspaceSearchSelect}
            />
          </div>
        {/if}
      </div>

      {#if FloatingControlsComponent}
        <FloatingControlsComponent
          labels={{
            title: $t('admin.network.map.floating.title') || 'Map controls',
            layers: $t('admin.network.map.floating.layers') || 'Layers',
            view: $t('admin.network.map.floating.view') || 'View',
            standard: $t('admin.network.map.view.standard') || 'Standard',
            satellite: $t('admin.network.map.view.satellite') || 'Satellite',
            nodes: $t('admin.network.map.stats.nodes') || 'Nodes',
            links: $t('admin.network.map.stats.links') || 'Links',
            zones: $t('admin.network.map.stats.zones') || 'Zones',
            assets: $t('admin.network.map.layers.assets') || 'FTTH Assets',
            routers: $t('admin.network.map.layers.routers') || 'Routers',
            customers: $t('admin.network.map.layers.customers') || 'Customers',
          }}
          hidden={controlsHidden}
          {viewMode}
          {nodesVisible}
          {linksVisible}
          {zonesVisible}
          {routersVisible}
          {customersVisible}
          {topologyAssetsVisible}
          canShowRouters={canReadRouterInventory}
          onViewModeChange={(mode: 'standard' | 'satellite') => (viewMode = mode)}
          onNodesVisibleChange={setNodesVisible}
          onLinksVisibleChange={setLinksVisible}
          onZonesVisibleChange={setZonesVisible}
          onRoutersVisibleChange={setRoutersVisible}
          onCustomersVisibleChange={setCustomersVisible}
          onTopologyAssetsVisibleChange={setTopologyAssetsVisible}
          onToggleHidden={() => (controlsHidden = !controlsHidden)}
        />
      {/if}

      {#if NodePanelComponent}
        <NodePanelComponent
          show={showCreateNodePanel}
          {editingNodeId}
          {nodePickMode}
          {savingNode}
          {nodeForm}
          {nodeTypeOptions}
          onClose={closeNodeModal}
          onSubmit={() => void submitNode()}
        />
      {/if}

      {#if linkPickMode}
        <div class="map-link-draw-controls">
          {#if linkPickDrawMode === 'path'}
            <button
              class="btn ghost btn-xs"
              type="button"
              onclick={undoLinkPathPoint}
              disabled={linkPathBendPoints.length === 0}
            >
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
    </svelte:fragment>
  </MapCanvasShell>
</div>

{#if LinkModalComponent}
  <LinkModalComponent
    show={showLinkModal}
    {editingLinkId}
    {savingLink}
    {linkPickDrawMode}
    {linkSnapToNodeEnabled}
    {linkPickMode}
    {linkPickStep}
    {linkPathBendPoints}
    {linkForm}
    {nodeRows}
    {linkTypeOptions}
    {linkStatusOptions}
    {linkFieldConfig}
    hasExistingLinkBetweenNodes={(
      fromNodeId: string,
      toNodeId: string,
      excludeLinkId?: string | null,
    ) => hasExistingLinkBetweenNodes(linkRows, fromNodeId, toNodeId, excludeLinkId)}
    onClose={closeLinkModal}
    onSubmit={() => void submitLink()}
    onTogglePickMode={toggleLinkPickMode}
    onSetDrawMode={setLinkPickDrawMode}
    onUndoPathPoint={undoLinkPathPoint}
    onClearPathPoints={clearLinkPathPoints}
    onUseStraightLine={useLinkFromNodePoints}
    onToggleSnap={() => (linkSnapToNodeEnabled = !linkSnapToNodeEnabled)}
  />
{/if}

{#if ZoneModalComponent}
  <ZoneModalComponent
    show={showZoneModal}
    {editingZoneId}
    {savingZone}
    {zoneForm}
    onClose={() => (showZoneModal = false)}
    onSubmit={() => void submitZone()}
  />
{/if}

{#if ConfirmDialogComponent}
  <ConfirmDialogComponent
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
{/if}

<style>
  .page-content {
    padding: 12px 14px 16px;
    max-width: 100%;
    margin: 0 auto;
  }

  .page-content.compact-mode {
    padding: 10px;
    max-width: 100%;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 12px;
    border-radius: 10px;
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

  .btn-xs {
    padding: 5px 9px;
    font-size: 0.78rem;
    border-radius: 8px;
  }

  .btn.danger {
    color: #fca5a5;
    border-color: color-mix(in srgb, #ef4444 55%, var(--border-color));
  }

  .map-workspace-search {
    position: absolute;
    top: 10px;
    left: 10px;
    z-index: 9;
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }

  .map-workspace-search-toggle {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border: 1px solid rgba(15, 23, 42, 0.22);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    cursor: pointer;
    box-shadow: 0 6px 14px rgba(15, 23, 42, 0.12);
  }

  .map-workspace-search-toggle.active {
    border-color: color-mix(in srgb, var(--color-primary) 54%, rgba(15, 23, 42, 0.22));
    color: var(--color-primary);
  }

  .map-workspace-search-panel {
    width: min(540px, calc(100vw - 92px));
    padding: 7px;
    border: 1px solid rgba(15, 23, 42, 0.14);
    border-radius: 10px;
    background: var(--bg-surface);
    box-shadow: 0 8px 18px rgba(15, 23, 42, 0.1);
  }

  .map-workspace-search-panel :global(.search-input-wrap) {
    background: var(--bg-surface);
    border-color: rgba(15, 23, 42, 0.2);
    box-shadow: none;
  }

  .map-workspace-search-panel :global(.search-input) {
    color: var(--text-primary);
  }

  .map-workspace-search-panel :global(.search-summary) {
    display: none;
  }

  .map-workspace-search-panel :global(.search-summary),
  .map-workspace-search-panel :global(.search-input::placeholder) {
    color: var(--text-secondary);
  }

  .map-workspace-search :global(.search-results) {
    z-index: 20;
    gap: 5px;
    max-height: min(50vh, 380px);
    padding: 7px;
    border-radius: 10px;
    background: var(--bg-surface);
    border-color: rgba(15, 23, 42, 0.16);
    box-shadow: 0 10px 22px rgba(15, 23, 42, 0.12);
    
  }

  .map-workspace-search :global(.search-group) {
    gap: 6px;
  }

  .map-workspace-search :global(.search-group-label) {
    padding: 0 4px;
    color: var(--text-secondary);
    font-size: 0.68rem;
    letter-spacing: 0.06em;
  }

  .map-workspace-search :global(.search-group-items) {
    gap: 5px;
  }

  .map-workspace-search :global(.search-item) {
    gap: 3px;
    min-height: 52px;
    padding: 8px 10px;
    border-color: rgba(15, 23, 42, 0.12);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    box-shadow: inset 3px 0 0 #94a3b8;
  }

  .map-workspace-search :global(.search-item:hover),
  .map-workspace-search :global(.search-item.active) {
    border-color: color-mix(in srgb, var(--color-primary) 42%, rgba(15, 23, 42, 0.12));
    background: color-mix(in srgb, var(--color-primary) 8%, var(--bg-surface));
    box-shadow:
      inset 3px 0 0 var(--color-primary),
      0 6px 14px rgba(15, 23, 42, 0.06);
  }

  .map-workspace-search :global(.search-item-label) {
    min-width: 0;
    overflow: hidden;
    font-size: 0.88rem;
    font-weight: 850;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .map-workspace-search :global(.search-item-kind) {
    flex: 0 0 auto;
    padding: 3px 7px;
    border-radius: 999px;
    background: #e2e8f0;
    color: #475569;
    font-size: 0.62rem;
    letter-spacing: 0;
    text-transform: capitalize;
  }

  .map-workspace-search :global(.search-item-subtitle) {
    overflow: hidden;
    color: var(--text-secondary);
    font-size: 0.78rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .map-workspace-search :global(.tone-ok) {
    box-shadow: inset 3px 0 0 #10b981;
  }

  .map-workspace-search :global(.tone-warn) {
    box-shadow: inset 3px 0 0 #f59e0b;
  }

  .map-workspace-search :global(.tone-muted) {
    box-shadow: inset 3px 0 0 #64748b;
  }

  .map-link-draw-controls {
    position: absolute;
    top: 12px;
    right: 58px;
    z-index: 8;
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 7px;
    border-radius: 9px;
    border: 1px solid var(--border-color, #24304a);
    background: var(--panel-bg, #0f1422);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.22);
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
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
    border-radius: 12px;
    box-shadow: 0 14px 32px rgba(0, 0, 0, 0.36);
    padding: 10px;
    min-width: 0;
    width: min(288px, calc(100vw - 44px));
    max-width: min(288px, calc(100vw - 44px)) !important;
    overflow: hidden;
  }

  :global(.maplibregl-popup.nm-popup-workflow-shell .maplibregl-popup-content) {
    width: min(332px, calc(100vw - 44px));
    max-width: min(332px, calc(100vw - 44px)) !important;
  }

  :global(.maplibregl-popup.nm-popup-link-shell .maplibregl-popup-content) {
    width: min(252px, calc(100vw - 44px));
    max-width: min(252px, calc(100vw - 44px)) !important;
    padding: 8px;
    border-color: rgba(59, 130, 246, 0.28);
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
    gap: 7px;
    max-height: min(430px, calc(100vh - 180px));
    overflow-y: auto;
    padding-right: 2px;
    padding-bottom: 2px;
    overscroll-behavior: contain;
  }

  :global(.nm-popup-card-workflow) {
    gap: 10px;
  }

  :global(.nm-popup-card-link) {
    gap: 8px;
    max-height: none;
    overflow: visible;
    padding-right: 0;
    padding-bottom: 0;
  }

  :global(.nm-popup-head) {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }

  :global(.nm-popup-kicker) {
    color: #93c5fd;
    font-size: 0.64rem;
    font-weight: 900;
    letter-spacing: 0.09em;
    text-transform: uppercase;
  }

  :global(.nm-popup-title) {
    margin-top: 2px;
    font-size: 0.93rem;
    font-weight: 900;
    color: #f8fafc;
    letter-spacing: 0.01em;
    line-height: 1.2;
  }

  :global(.nm-popup-card-link .nm-popup-title) {
    max-width: 160px;
    overflow: hidden;
    color: #f8fafc;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.nm-popup-subtitle) {
    margin-top: 3px;
    color: #cbd5e1;
    font-size: 0.76rem;
    line-height: 1.28;
  }

  :global(.nm-popup-card-link .nm-popup-subtitle) {
    color: #93c5fd;
    font-size: 0.74rem;
    font-weight: 700;
    text-transform: capitalize;
  }

  :global(.nm-popup-badge) {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 4px 9px;
    font-size: 0.66rem;
    font-weight: 900;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border: 1px solid transparent;
    white-space: nowrap;
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
    grid-template-columns: 78px minmax(0, 1fr);
    gap: 5px 10px;
  }

  :global(.nm-popup-context) {
    border-radius: 12px;
    padding: 8px 10px;
    background: rgba(15, 23, 42, 0.78);
    color: #e2e8f0;
    font-size: 0.77rem;
    line-height: 1.4;
    border: 1px solid rgba(148, 163, 184, 0.14);
  }

  :global(.nm-popup-context-workflow) {
    background: rgba(8, 47, 73, 0.36);
    border-color: rgba(56, 189, 248, 0.2);
    color: #dbeafe;
    font-weight: 600;
  }

  :global(.nm-popup-status-chips) {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  :global(.nm-popup-status-chips-workflow) {
    gap: 8px;
  }

  :global(.nm-popup-status-chip) {
    border-radius: 12px;
    padding: 8px 10px;
    border: 1px solid rgba(148, 163, 184, 0.18);
    background: rgba(15, 23, 42, 0.74);
  }

  :global(.nm-popup-status-chip.ok) {
    border-color: rgba(34, 197, 94, 0.3);
    background: rgba(20, 83, 45, 0.26);
  }

  :global(.nm-popup-status-chip.warn) {
    border-color: rgba(245, 158, 11, 0.3);
    background: rgba(120, 53, 15, 0.26);
  }

  :global(.nm-popup-status-chip.muted) {
    border-color: rgba(100, 116, 139, 0.26);
    background: rgba(15, 23, 42, 0.88);
  }

  :global(.nm-popup-status-chip.danger) {
    border-color: rgba(248, 113, 113, 0.34);
    background: rgba(127, 29, 29, 0.3);
  }

  :global(.nm-popup-status-chip-label) {
    color: #94a3b8;
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 800;
  }

  :global(.nm-popup-status-chip-value) {
    margin-top: 3px;
    color: #f8fafc;
    font-size: 0.78rem;
    font-weight: 900;
    line-height: 1.2;
  }

  :global(.nm-popup-summary) {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 7px;
  }

  :global(.nm-popup-summary-workflow) {
    gap: 8px;
  }

  :global(.nm-popup-summary-link) {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  :global(.nm-popup-summary-item) {
    border-radius: 12px;
    padding: 9px 10px;
    background: rgba(15, 23, 42, 0.72);
    border: 1px solid rgba(148, 163, 184, 0.14);
  }

  :global(.nm-popup-summary-item.ok) {
    border-color: rgba(34, 197, 94, 0.28);
    background: rgba(21, 128, 61, 0.14);
  }

  :global(.nm-popup-summary-item.warn) {
    border-color: rgba(245, 158, 11, 0.28);
    background: rgba(180, 83, 9, 0.14);
  }

  :global(.nm-popup-summary-item.danger) {
    border-color: rgba(248, 113, 113, 0.3);
    background: rgba(127, 29, 29, 0.18);
  }

  :global(.nm-popup-summary-label) {
    color: #94a3b8;
    font-size: 0.64rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 800;
  }

  :global(.nm-popup-summary-value) {
    margin-top: 3px;
    color: #f8fafc;
    font-size: 0.84rem;
    font-weight: 900;
    line-height: 1.2;
    word-break: break-word;
  }

  :global(.nm-popup-card-link .nm-popup-summary-item) {
    min-height: 58px;
    border-radius: 10px;
    padding: 8px 9px;
  }

  :global(.nm-popup-card-link .nm-popup-summary-label) {
    font-size: 0.62rem;
  }

  :global(.nm-popup-card-link .nm-popup-summary-value) {
    font-size: 1rem;
  }

  :global(.nm-popup-label) {
    color: #94a3b8;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 800;
  }

  :global(.nm-popup-value) {
    color: #e2e8f0;
    font-size: 0.8rem;
    font-weight: 600;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }

  :global(.nm-popup-value.mono) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.78rem;
  }

  :global(.nm-popup-actions) {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 7px;
    margin-top: 6px;
    padding-top: 7px;
    padding-bottom: 2px;
    border-top: 1px solid rgba(148, 163, 184, 0.2);
  }

  :global(.nm-popup-actions-workflow) {
    justify-content: flex-start;
    flex-wrap: nowrap;
    gap: 5px;
  }

  :global(.nm-popup-actions-workflow .nm-popup-btn.primary) {
    flex: 1 1 auto;
    min-width: 0;
    order: -1;
  }

  :global(.nm-popup-actions-workflow .nm-popup-btn) {
    min-height: 28px;
    padding: 5px 7px;
    font-size: 0.69rem;
    line-height: 1.1;
    white-space: nowrap;
  }

  :global(.nm-popup-actions-workflow .nm-popup-btn:not(.primary)) {
    flex: 0 0 auto;
  }

  :global(.nm-popup-actions-workflow .action-open-service) {
    border-color: rgba(56, 189, 248, 0.34);
    background: rgba(56, 189, 248, 0.1);
    color: #dbeafe;
  }

  :global(.nm-popup-actions-workflow .action-connect) {
    border-color: rgba(148, 163, 184, 0.26);
    background: rgba(15, 23, 42, 0.72);
    color: #cbd5e1;
  }

  :global(.nm-popup-actions-workflow .nm-popup-btn-close) {
    margin-left: auto;
    border-color: rgba(148, 163, 184, 0.18);
    background: transparent;
    color: #94a3b8;
  }

  :global(.nm-popup-actions-link) {
    margin-top: 2px;
    padding-top: 8px;
    justify-content: flex-end;
    gap: 6px;
  }

  :global(.nm-popup-actions-link .nm-popup-btn) {
    min-height: 30px;
    padding: 5px 10px;
    border-radius: 8px;
    font-size: 0.72rem;
  }

  :global(.nm-popup-actions-link .nm-popup-btn-close) {
    border-color: rgba(148, 163, 184, 0.24);
    background: transparent;
    color: #cbd5e1;
  }

  :global(.nm-popup-btn) {
    min-height: 32px;
    padding: 6px 10px;
    border-radius: 9px;
    border: 1px solid #475569;
    background: #0b1322;
    color: #e2e8f0;
    font-size: 0.74rem;
    font-weight: 700;
    cursor: pointer;
  }

  :global(.nm-popup-card::-webkit-scrollbar) {
    width: 6px;
  }

  :global(.nm-popup-card::-webkit-scrollbar-thumb) {
    background: rgba(148, 163, 184, 0.4);
    border-radius: 999px;
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

  @media (max-width: 640px) {
    :global(.maplibregl-popup-content) {
      padding: 8px;
      width: min(264px, calc(100vw - 28px));
      max-width: min(264px, calc(100vw - 28px)) !important;
      border-radius: 10px;
    }

    :global(.maplibregl-popup.nm-popup-workflow-shell .maplibregl-popup-content) {
      width: min(264px, calc(100vw - 28px));
      max-width: min(264px, calc(100vw - 28px)) !important;
    }

    :global(.nm-popup-card) {
      gap: 6px;
      max-height: min(68dvh, calc(100dvh - 108px));
      padding-right: 0;
      padding-bottom: max(10px, env(safe-area-inset-bottom, 0px));
    }

    :global(.nm-popup-card-workflow) {
      gap: 7px;
    }

    :global(.nm-popup-head) {
      flex-direction: column;
      align-items: stretch;
      gap: 6px;
    }

    :global(.nm-popup-badge) {
      align-self: flex-start;
      padding: 3px 8px;
      font-size: 0.62rem;
    }

    :global(.nm-popup-title) {
      font-size: 0.88rem;
    }

    :global(.nm-popup-subtitle) {
      font-size: 0.72rem;
    }

    :global(.nm-popup-status-chips) {
      grid-template-columns: minmax(0, 1fr);
      gap: 6px;
    }

    :global(.nm-popup-summary) {
      grid-template-columns: minmax(0, 1fr);
      gap: 6px;
    }

    :global(.nm-popup-summary-item),
    :global(.nm-popup-status-chip),
    :global(.nm-popup-context) {
      border-radius: 10px;
      padding: 8px 9px;
    }

    :global(.nm-popup-grid) {
      grid-template-columns: minmax(0, 1fr);
      gap: 3px;
    }

    :global(.nm-popup-label) {
      margin-top: 2px;
      font-size: 0.64rem;
    }

    :global(.nm-popup-value) {
      margin-bottom: 2px;
      font-size: 0.76rem;
    }

    :global(.nm-popup-actions) {
      gap: 6px;
      margin-top: 4px;
      padding-top: 8px;
      padding-bottom: max(4px, env(safe-area-inset-bottom, 0px));
    }

    :global(.nm-popup-actions-workflow) {
      flex-wrap: wrap;
      gap: 6px;
      position: sticky;
      bottom: 0;
      z-index: 2;
      background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
    }

    :global(.nm-popup-actions-workflow .action-connect) {
      display: none;
    }

    :global(.nm-popup-actions-workflow .nm-popup-btn) {
      min-height: 32px;
      padding: 6px 9px;
      font-size: 0.71rem;
      white-space: normal;
      text-align: center;
      justify-content: center;
    }

    :global(.nm-popup-actions-workflow .nm-popup-btn.primary),
    :global(.nm-popup-actions-workflow .action-open-service),
    :global(.nm-popup-actions-workflow .action-open-customer) {
      flex: 1 1 calc(50% - 3px);
      min-width: 0;
      order: 0;
    }

    :global(.nm-popup-actions-workflow .nm-popup-btn-close) {
      flex: 1 1 100%;
      margin-left: 0;
    }
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 12px;
    }

    .map-workspace-search {
      top: 12px;
      right: 12px;
    }

    .map-workspace-search-panel {
      width: calc(100vw - 82px);
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
  }

  @media (max-width: 560px) {
    .page-content {
      padding: 8px;
    }

    .page-content :global(.network-page-actions) {
      grid-template-columns: 1fr;
    }

  }
</style>
