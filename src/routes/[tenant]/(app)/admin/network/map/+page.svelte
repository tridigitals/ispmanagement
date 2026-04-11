<script lang="ts">
  import type { Geometry } from 'geojson';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onDestroy, onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { can, tenant, user } from '$lib/stores/auth';
  import { api, type PaginatedResponse } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import NetworkMapLinkModal from '$lib/components/network/NetworkMapLinkModal.svelte';
  import NetworkMapManager from '$lib/components/network/NetworkMapManager.svelte';
  import NetworkMapNodePanel from '$lib/components/network/NetworkMapNodePanel.svelte';
  import NetworkMapOverview from '$lib/components/network/NetworkMapOverview.svelte';
  import NetworkMapFloatingControls from '$lib/components/network/NetworkMapFloatingControls.svelte';
  import NetworkMapZoneModal from '$lib/components/network/NetworkMapZoneModal.svelte';
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
    buildDefaultLineGeometry,
    buildDeleteConfirmCopy,
    currentDraftPathCoords,
    hasExistingLinkBetweenNodes,
  } from '$lib/components/network/networkMapInteractionUtils';
  import { handleCanvasMapClick } from '$lib/components/network/networkMapCanvasInteractions';
  import {
    createZoneBindingCrud,
    loadZoneBindingsCrud,
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
    type LinkPickDrawMode,
  } from '$lib/components/network/networkMapLinkPicking';
  import {
    openLinkPopup,
    openNodePopup,
    openRouterPopup,
  } from '$lib/components/network/networkMapPopups';
  import {
    applyCachedMapData,
    applyFetchedMapData,
    buildMapDataCacheKey,
    fetchNetworkMapData,
    getCachedMapData,
    setCachedMapData,
    syncTopologyAssetsIfNeeded,
    type NetworkMapCacheEntry,
  } from '$lib/components/network/networkMapData';
  import {
    buildBaseMapStyle,
    emptyFeatureCollection,
    registerMapSourcesAndLayers,
    SOURCE_CUSTOMERS,
    SOURCE_LINK_DRAFT,
    SOURCE_LINK_DRAFT_POINTS,
    SOURCE_LINKS,
    SOURCE_NODES,
    SOURCE_ROUTERS,
    SOURCE_SELECTION_LINES,
    SOURCE_SELECTION_POINTS,
    SOURCE_SELECTION_ZONES,
    SOURCE_ZONES,
  } from '$lib/components/network/networkMapLayers';
  import {
    expandCustomerCluster,
    registerInteractiveLayerHover,
    registerPrimaryLayerClicks,
  } from '$lib/components/network/networkMapInit';
  import {
    emitInstallationRefreshSignal,
    emitWorkOrderUpdatedToParent,
    resolveInstallationTargetMarker,
  } from '$lib/components/network/networkMapInstallation';
  import { fitMapToMarkers } from '$lib/components/network/networkMapRuntime';
  import {
    asNumber,
    customersToFeatureCollection,
    ensureNodeTypeIconsRegistered,
    filterRoutersForOverlay,
    getLinkFieldConfig,
    isCustomerNodeType,
    isSystemManagedNode,
    linkStatusOptions,
    linkTypeOptions,
    linksToFeatureCollection,
    nodeTypeOptions,
    nodesToFeatureCollection,
    parseGeometryText,
    prettyGeometry,
    routersToFeatureCollection,
    systemManagedNodeSourceLabel,
    zonesToFeatureCollection,
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
    clearWorkspaceSelection,
    createNetworkMapWorkspaceState,
    enterInvestigationMode,
    selectNetworkMapObject,
    type NetworkMapWorkspaceState,
    type NetworkMapWorkspaceCapabilities,
  } from '$lib/components/network/networkMapWorkspaceState';
  import {
    groupNetworkMapSearchResults,
    type NetworkMapSearchResultItem,
  } from '$lib/components/network/networkMapInsights';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type MaplibreModule = typeof import('maplibre-gl');
  type MapInstance = import('maplibre-gl').Map;
  type NetworkMapQuickMode = 'all' | 'issues' | 'customers' | 'services' | 'topology' | 'field';

  let mapEl = $state<HTMLDivElement | null>(null);
  let map = $state<MapInstance | null>(null);
  let maplibre = $state<MaplibreModule | null>(null);
  let mapReady = $state(false);
  let mapUnavailable = $state(false);
  let mapErrorMessage = $state('');
  let loading = $state(true);
  let refreshing = $state(false);
  let syncingAssetNodes = $state(false);

  let nodesVisible = $state(true);
  let linksVisible = $state(true);
  let zonesVisible = $state(true);
  let routersVisible = $state(true);
  let customersVisible = $state(true);
  let viewMode = $state<'standard' | 'satellite'>('standard');
  let controlsHidden = $state(true);

  let q = $state('');
  let workspaceSearchQuery = $state('');
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
  let customerRows = $state<NMNode[]>([]);
  let serviceRows = $state<NMNode[]>([]);
  let zoneBindings = $state<any[]>([]);
  let selectedZoneId = $state('');
  let selectedTab = $state<'nodes' | 'links' | 'zones' | 'bindings'>('nodes');
  let manageMode = $state(false);
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

  let bindingForm = $state({
    zone_id: '',
    node_id: '',
    is_primary: false,
    weight: '100',
  });
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
  let didInitialFitToMarkers = false;
  let lastAssetSyncAt = 0;
  let lastMapDataLoadedAt = $state(0);
  let lastMapDataSource = $state<'live' | 'cache' | 'none'>('none');
  let currentTimeMs = $state(Date.now());
  const dataCache = new Map<string, NetworkMapCacheEntry>();
  const dataCacheTtlMs = 20_000;
  const dataCacheMaxEntries = 40;
  const assetSyncTtlMs = 45_000;
  const mapTilerKey = (import.meta.env.VITE_MAPTILER_KEY as string | undefined)?.trim();
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
  const searchGroups = $derived.by(() => {
    const groups = groupNetworkMapSearchResults({
      query: workspaceSearchQuery,
      nodes: nodeRows,
      links: linkRows,
      zones: zoneRows,
      routers: routerRows,
      customerRows,
      serviceRows,
    });

    if (quickMode === 'all') return groups;

    const allowedGroupKeys: Record<Exclude<NetworkMapQuickMode, 'all'>, string[]> = {
      issues: ['nodes', 'links', 'zones', 'routers'],
      customers: ['customers'],
      services: ['services'],
      topology: ['nodes', 'links', 'zones', 'routers'],
      field: ['customers', 'services', 'nodes', 'zones'],
    };

    const allowedKeys = allowedGroupKeys[quickMode as Exclude<NetworkMapQuickMode, 'all'>] || [];
    return groups.filter((group) => allowedKeys.includes(group.key));
  });
  const searchResultCount = $derived.by(() =>
    searchGroups.reduce((total, group) => total + group.items.length, 0),
  );
  const searchSummary = $derived.by(() => {
    const query = workspaceSearchQuery.trim();
    if (!query) {
      return (
        $t('admin.network.map.search.summary_idle', {
          values: {
            count: nodeRows.length + linkRows.length + zoneRows.length + routerRows.length,
          },
        }) ||
        `${nodeRows.length + linkRows.length + zoneRows.length + routerRows.length} assets loaded in this workspace`
      );
    }

    return (
      $t('admin.network.map.search.summary_results', {
        values: {
          count: searchResultCount,
          groups: searchGroups.length,
        },
      }) || `${searchResultCount} matching results across ${searchGroups.length} sections`
    );
  });
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
  const installationReturnUrl = $derived.by(() => {
    if (!fromInstallation) return '';
    const params = new URLSearchParams();
    if (sourceWorkOrderId) params.set('work_order_id', sourceWorkOrderId);
    return `${tenantPrefix}/admin/network/installations${params.toString() ? `?${params.toString()}` : ''}`;
  });

  onMount(() => {
    if (!$can('read', 'network_topology') && !$can('manage', 'network_topology')) {
      goto('/unauthorized');
      return;
    }
    if (typeof window !== 'undefined') {
      freshnessTimer = setInterval(() => {
        currentTimeMs = Date.now();
      }, 15_000);
    }
    workspaceState = applyNetworkMapWorkspaceDefaults(workspaceState, workspaceDefaults);
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

  function updateWorkspaceSelection(nextSelectedObject: ReturnType<typeof buildSelectedMapObject>) {
    workspaceState = selectNetworkMapObject(workspaceState, nextSelectedObject);
    if (workspaceDefaults.mode === 'investigate') {
      workspaceState = enterInvestigationMode(workspaceState, workspaceDefaults.investigationKind);
    }
  }

  function clearMapPopupSelection() {
    workspaceState = clearWorkspaceSelection(workspaceState);
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

  function applyQuickMode(nextMode: NetworkMapQuickMode) {
    quickMode = nextMode;
    if (nextMode === 'all') {
      nodesVisible = true;
      linksVisible = true;
      zonesVisible = true;
      routersVisible = true;
      customersVisible = true;
      return;
    }

    if (nextMode === 'issues') {
      nodesVisible = true;
      linksVisible = true;
      zonesVisible = true;
      routersVisible = canReadRouterInventory;
      customersVisible = false;
      return;
    }

    if (nextMode === 'customers') {
      nodesVisible = false;
      linksVisible = false;
      zonesVisible = false;
      routersVisible = false;
      customersVisible = true;
      return;
    }

    if (nextMode === 'services') {
      nodesVisible = true;
      linksVisible = false;
      zonesVisible = false;
      routersVisible = false;
      customersVisible = true;
      return;
    }

    if (nextMode === 'topology') {
      nodesVisible = true;
      linksVisible = true;
      zonesVisible = true;
      routersVisible = canReadRouterInventory;
      customersVisible = false;
      return;
    }

    nodesVisible = true;
    linksVisible = false;
    zonesVisible = true;
    routersVisible = false;
    customersVisible = true;
  }

  function openManageWorkspace(tab: 'nodes' | 'links' | 'zones' | 'bindings') {
    if (!canManageTopology) return;
    selectedTab = tab;
    manageMode = true;
  }

  function closeManageWorkspace() {
    manageMode = false;
  }

  function handleWorkspaceSearchSelect(item: NetworkMapSearchResultItem) {
    workspaceSearchQuery = item.label;

    if (item.kind === 'customer' || item.kind === 'service' || item.kind === 'node') {
      const row = nodeRows.find((candidate) => candidate.id === item.id);
      if (!row) return;
      focusMapOnCoordinates(row.lng, row.lat, 14);
      updateWorkspaceSelection(
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
      updateWorkspaceSelection(
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
      updateWorkspaceSelection(
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
    updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'router',
        id: router.id,
        label: router.identity || router.name,
      }),
    );
  }

  function handleNodeLayerClick(e: any) {
    if (!map || !e.features?.[0] || !maplibre) return;
    const props = e.features[0].properties || {};
    const nodeId = String(props.id || '');
    updateWorkspaceSelection(
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
    openNodePopup({
      map,
      maplibre,
      feature: e.features[0],
      nodeRows,
      routerRows,
      activePopup: activeNodePopup,
      setActivePopup: (popup) => (activeNodePopup = popup),
      onClose: clearMapPopupSelection,
      onConnect: startConnectFromNode,
      onEdit: openEditNodeModal,
      onOpenRouter: (routerId) => void goto(`${tenantPrefix}/admin/network/routers/${routerId}`),
    });
  }

  function handleLinkLayerClick(e: any) {
    if (!map || !e.features?.[0] || !maplibre || linkPickMode) return;
    const props = e.features[0].properties || {};
    const linkId = String(props.id || '');
    updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'link',
        id: linkId,
        label: String(props.name || props.label || linkId),
        linkType: props.link_type || props.linkType || undefined,
      }),
    );
    openLinkPopup({
      map,
      maplibre,
      feature: e.features[0],
      lngLat: e.lngLat,
      linkRows,
      onClose: clearMapPopupSelection,
      onDelete: (linkId, linkName) => openDeleteConfirm('link', linkId, linkName),
    });
  }

  function handleRouterLayerClick(e: any) {
    if (!map || !e.features?.[0] || !maplibre) return;
    const props = e.features[0].properties || {};
    const routerId = String(props.id || '');
    updateWorkspaceSelection(
      buildSelectedMapObject({
        kind: 'router',
        id: routerId,
        label: String(props.name || props.identity || routerId),
      }),
    );
    openRouterPopup({
      map,
      maplibre,
      feature: e.features[0],
      activePopup: activeNodePopup,
      setActivePopup: (popup) => (activeNodePopup = popup),
      onClose: clearMapPopupSelection,
      onOpenRouter: (routerId) => void goto(`${tenantPrefix}/admin/network/routers/${routerId}`),
    });
  }

  async function initMap() {
    try {
      maplibre = await import('maplibre-gl');
      if (!mapEl || !maplibre) return;

      map = new maplibre.Map({
        container: mapEl,
        style: buildBaseMapStyle({
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
        ensureNodeTypeIconsRegistered(map);
        registerMapSourcesAndLayers(map);

        registerPrimaryLayerClicks({
          map,
          onNodeClick: handleNodeLayerClick,
          onRouterClick: handleRouterLayerClick,
          onLinkClick: handleLinkLayerClick,
          onCustomerClusterClick: async (e) => {
            if (!map || !maplibre || !e.features?.[0]) return;
            try {
              await expandCustomerCluster({
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
          const result = handleCanvasMapClick({
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

        registerInteractiveLayerHover(map);

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
              loadCustomerLocations: (customerId) => api.customers.locations.list(customerId),
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

  async function refreshMapData(force = false) {
    if (map && !mapReady) return;
    const requestId = ++lastRequestId;
    const bbox = currentBboxString();
    if (!bbox) return;

    refreshing = true;

    try {
      let shouldBypassCache = force;
      if (await syncTopologyAssets(force)) {
        shouldBypassCache = true;
        invalidateMapDataCache();
      }

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
        return;
      }

      activeDataAbortController?.abort();
      const abortController = new AbortController();
      activeDataAbortController = abortController;

      const result = await fetchNetworkMapData(params, abortController.signal, {
        includeRouters: canReadRouterInventory,
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
    } catch (e: any) {
      if ((e?.message || '').includes('Request canceled')) return;
      console.error(e);
    } finally {
      if (requestId === lastRequestId) activeDataAbortController = null;
      refreshing = false;
    }
  }

  function fitMapToAllMarkersOnFirstLoad(nodes: NMNode[], routers: NMRouter[]) {
    if (!map || !maplibre) return;
    const didFit = fitMapToMarkers({
      map,
      maplibre,
      didInitialFitToMarkers,
      nodes,
      routers,
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

  function syncWorkspaceHighlights() {
    if (!map || !mapReady) return;

    const selectedObject =
      workspaceState.investigationState?.selectedObject || workspaceState.selectedObject;

    let pointData = emptyFeatureCollection();
    let lineData = emptyFeatureCollection();
    let zoneData = emptyFeatureCollection();

    if (selectedObject) {
      if (
        selectedObject.kind === 'node' ||
        selectedObject.kind === 'customer' ||
        selectedObject.kind === 'service'
      ) {
        const row = nodeRows.find((candidate) => candidate.id === selectedObject.id);
        if (row) pointData = nodesToFeatureCollection([row]);
      } else if (selectedObject.kind === 'link') {
        const row = linkRows.find((candidate) => candidate.id === selectedObject.id);
        if (row) lineData = linksToFeatureCollection([row]);
      } else if (selectedObject.kind === 'zone') {
        const row = zoneRows.find((candidate) => candidate.id === selectedObject.id);
        if (row) zoneData = zonesToFeatureCollection([row]);
      } else if (selectedObject.kind === 'router') {
        const row = routerRows.find((candidate) => candidate.id === selectedObject.id);
        if (row) pointData = routersToFeatureCollection([row]);
      }
    }

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
    setLayerVisibility('nm-nodes-circle', nodesVisible);
    setLayerVisibility('nm-nodes-icons', nodesVisible);
    setLayerVisibility('nm-routers-circle', routersVisible);
    setLayerVisibility('nm-routers-icon', routersVisible);
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
    manageMode = true;
    selectedTab = 'nodes';
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
      await loadZoneBindings();
    } finally {
      savingNode = false;
    }
  }

  function openCreateLinkModal() {
    manageMode = true;
    selectedTab = 'links';
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

  function startConnectFromNode(nodeId: string) {
    activeNodePopup?.remove();
    const next = buildConnectFromNodeResult(nodeId, nodeRows);
    const nodeRow = nodeRows.find((row) => row.id === nodeId);
    updateWorkspaceSelection(
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
    selectedTab = 'links';
    toast.info(next.toastMessage);
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
      const ok = await submitLinkCrud({
        editingLinkId,
        linkForm,
        linkFieldConfig,
        hasExistingLinkBetweenNodes: (fromNodeId, toNodeId, excludeLinkId) =>
          hasExistingLinkBetweenNodes(linkRows, fromNodeId, toNodeId, excludeLinkId),
      });
      if (!ok) return;
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
    manageMode = true;
    selectedTab = 'zones';
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
      await loadZoneBindings();
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
      await loadZoneBindings();
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
      if (selectedZoneId === id) selectedZoneId = '';
      invalidateMapDataCache();
      await refreshMapData(true);
      await loadZoneBindings();
    } finally {
      deletingId = null;
    }
  }

  async function loadZoneBindings() {
    loadingManager = true;
    try {
      const rows = await loadZoneBindingsCrud(selectedZoneId);
      if (rows) zoneBindings = rows;
    } finally {
      loadingManager = false;
    }
  }

  async function createZoneBinding() {
    savingBinding = true;
    try {
      const ok = await createZoneBindingCrud(bindingForm);
      if (!ok) return;
      bindingForm = { zone_id: bindingForm.zone_id, node_id: '', is_primary: false, weight: '100' };
      await loadZoneBindings();
    } finally {
      savingBinding = false;
    }
  }

  async function removeBinding(id: string) {
    deletingId = id;
    try {
      const ok = await removeCrud({ type: 'binding', id });
      if (!ok) return;
      await loadZoneBindings();
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

<div class="page-content fade-in" class:compact-mode={compactMode}>
  <NetworkMapOverview
    {compactMode}
    {fromInstallation}
    {installationReturnUrl}
    {tenantPrefix}
    {canManageTopology}
    {syncingAssetNodes}
    {refreshing}
    {loading}
    {workspaceSearchQuery}
    {searchGroups}
    {searchSummary}
    title={$t('admin.network.map.title') || 'Network Topology Map'}
    subtitle={workspaceSubtitle}
    labels={{
      backToInstallation: $t('admin.network.map.back_to_installation') || 'Back to Installation',
      backToNoc: $t('admin.network.map.back_to_noc') || 'Back to NOC',
      searchKicker: $t('admin.network.map.search.kicker') || 'Unified search',
      searchTitle:
        $t('admin.network.map.search.title') || 'Jump to any mapped asset, service, or customer',
      searchHint:
        $t('admin.network.map.search.hint') ||
        'Search across infrastructure, customer endpoints, services, zones, and router inventory.',
      searchPlaceholder:
        $t('admin.network.map.search.placeholder') ||
        'Search customer, service, node, link, zone, or router...',
      searchEmptyTitle: $t('admin.network.map.search.empty_title') || 'No matching results',
      searchEmptyHint:
        $t('admin.network.map.search.empty_hint') ||
        'Try another keyword to widen the result scope.',
      syncing: 'Syncing...',
      syncAssets: 'Sync Router & Customer Nodes',
      loading: $t('common.loading') || 'Loading...',
      refresh: $t('common.refresh') || 'Refresh',
      nodes: $t('admin.network.map.stats.nodes') || 'Nodes',
      links: $t('admin.network.map.stats.links') || 'Links',
      zones: $t('admin.network.map.stats.zones') || 'Zones',
      routers: $t('admin.network.map.layers.routers') || 'Routers',
      customers: $t('admin.network.map.layers.customers') || 'Customers',
    }}
    onWorkspaceSearchChange={(value) => (workspaceSearchQuery = value)}
    onWorkspaceSearchSelect={handleWorkspaceSearchSelect}
    onSyncAssets={async () => {
      if (await syncTopologyAssets(true)) {
        invalidateMapDataCache();
      }
      await refreshMapData(true);
    }}
    onRefresh={() => void refreshMapData()}
  />

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
    height={compactMode ? 'min(76vh, 760px)' : 'min(62vh, 700px)'}
  >
    <svelte:fragment slot="overlay">
      <NetworkMapFloatingControls
        labels={{
          title: $t('admin.network.map.floating.title') || 'Map controls',
          layers: $t('admin.network.map.floating.layers') || 'Layers',
          view: $t('admin.network.map.floating.view') || 'View',
          manage: $t('admin.network.map.floating.manage') || 'Manage',
          standard: $t('admin.network.map.view.standard') || 'Standard',
          satellite: $t('admin.network.map.view.satellite') || 'Satellite',
          openNodes: $t('admin.network.map.floating.open_nodes') || 'Manage nodes',
          openLinks: $t('admin.network.map.floating.open_links') || 'Manage links',
          openZones: $t('admin.network.map.floating.open_zones') || 'Manage zones',
          openBindings: $t('admin.network.map.floating.open_bindings') || 'Manage bindings',
          addNode: $t('admin.network.map.floating.add_node') || 'Add node',
          addLink: $t('admin.network.map.floating.add_link') || 'Add link',
          addZone: $t('admin.network.map.floating.add_zone') || 'Add zone',
          nodes: $t('admin.network.map.stats.nodes') || 'Nodes',
          links: $t('admin.network.map.stats.links') || 'Links',
          zones: $t('admin.network.map.stats.zones') || 'Zones',
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
        canShowRouters={canReadRouterInventory}
        {canManageTopology}
        onViewModeChange={(mode) => (viewMode = mode)}
        onNodesVisibleChange={(checked) => (nodesVisible = checked)}
        onLinksVisibleChange={(checked) => (linksVisible = checked)}
        onZonesVisibleChange={(checked) => (zonesVisible = checked)}
        onRoutersVisibleChange={(checked) => (routersVisible = checked)}
        onCustomersVisibleChange={(checked) => (customersVisible = checked)}
        onOpenManageNodes={() => openManageWorkspace('nodes')}
        onOpenManageLinks={() => openManageWorkspace('links')}
        onOpenManageZones={() => openManageWorkspace('zones')}
        onOpenManageBindings={() => openManageWorkspace('bindings')}
        onToggleHidden={() => (controlsHidden = !controlsHidden)}
      />

      <NetworkMapNodePanel
        show={showCreateNodePanel}
        {editingNodeId}
        {nodePickMode}
        {savingNode}
        {nodeForm}
        {nodeTypeOptions}
        onClose={closeNodeModal}
        onSubmit={() => void submitNode()}
      />

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

  {#if !compactMode}
    <NetworkMapManager
      {manageMode}
      title={$t('admin.network.map.manage.title') || 'Topology editor workspace'}
      subtitle={$t('admin.network.map.manage.subtitle') ||
        'Editing lives in a separate workspace so monitoring and investigation stay focused.'}
      {selectedTab}
      {nodeRows}
      {linkRows}
      {zoneRows}
      {zoneBindings}
      {selectedZoneId}
      {loadingManager}
      {savingBinding}
      {deletingId}
      {bindingForm}
      onClose={closeManageWorkspace}
      onSelectTab={(tab) => (selectedTab = tab)}
      onOpenCreateNode={openCreateNodeModal}
      onOpenCreateLink={openCreateLinkModal}
      onOpenCreateZone={openCreateZoneModal}
      onStartConnectNode={startConnectFromNode}
      onOpenEditNode={openEditNodeModal}
      onOpenEditLink={openEditLinkModal}
      onOpenEditZone={openEditZoneModal}
      onOpenDeleteConfirm={openDeleteConfirm}
      onSelectedZoneChange={(value) => (selectedZoneId = value)}
      onBindingNodeChange={(value) => (bindingForm = { ...bindingForm, node_id: value })}
      onBindingWeightChange={(value) => (bindingForm = { ...bindingForm, weight: value })}
      onBindingPrimaryChange={(checked) => (bindingForm = { ...bindingForm, is_primary: checked })}
      onCreateBinding={() => void createZoneBinding()}
    />
  {/if}
</div>

<NetworkMapLinkModal
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
  hasExistingLinkBetweenNodes={(fromNodeId, toNodeId, excludeLinkId) =>
    hasExistingLinkBetweenNodes(linkRows, fromNodeId, toNodeId, excludeLinkId)}
  onClose={closeLinkModal}
  onSubmit={() => void submitLink()}
  onTogglePickMode={toggleLinkPickMode}
  onSetDrawMode={setLinkPickDrawMode}
  onUndoPathPoint={undoLinkPathPoint}
  onClearPathPoints={clearLinkPathPoints}
  onUseStraightLine={useLinkFromNodePoints}
  onToggleSnap={() => (linkSnapToNodeEnabled = !linkSnapToNodeEnabled)}
/>

<NetworkMapZoneModal
  show={showZoneModal}
  {editingZoneId}
  {savingZone}
  {zoneForm}
  onClose={() => (showZoneModal = false)}
  onSubmit={() => void submitZone()}
/>

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

  .page-content.compact-mode {
    padding: 10px;
    max-width: 100%;
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

  .btn-xs {
    padding: 6px 10px;
    font-size: 0.78rem;
    border-radius: 9px;
  }

  .btn.danger {
    color: #fca5a5;
    border-color: color-mix(in srgb, #ef4444 55%, var(--border-color));
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

  :global(.nm-popup-subtitle) {
    margin-top: 3px;
    color: #cbd5e1;
    font-size: 0.76rem;
    line-height: 1.28;
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

  :global(.nm-popup-summary) {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 7px;
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
    border-top: 1px solid rgba(148, 163, 184, 0.2);
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
  }

  @media (max-width: 560px) {
    .page-content :global(.network-page-actions) {
      grid-template-columns: 1fr;
    }
  }
</style>
