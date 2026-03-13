<script lang="ts">
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
  import { openLinkPopup, openNodePopup, openRouterPopup } from '$lib/components/network/networkMapPopups';
  import {
    applyCachedMapData,
    applyFetchedMapData,
    buildMapDataCacheKey,
    extractMapRows,
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
    SOURCE_ZONES,
  } from '$lib/components/network/networkMapLayers';
  import {
    createMapTextButtonControl,
    expandCustomerCluster,
    registerInteractiveLayerHover,
    registerPrimaryLayerClicks,
  } from '$lib/components/network/networkMapInit';
  import {
    emitInstallationRefreshSignal,
    emitWorkOrderUpdatedToParent,
    resolveInstallationTargetMarker,
  } from '$lib/components/network/networkMapInstallation';
  import {
    fitMapToMarkers,
    hideMyLocationMarker,
    showMyLocationMarker,
    syncMyLocationControlButton,
    syncViewModeControlButton,
  } from '$lib/components/network/networkMapRuntime';
  import {
    asNumber,
    computeLinkHealth,
    customersToFeatureCollection,
    ensureNodeTypeIconsRegistered,
    filterRoutersForOverlay,
    getLinkFieldConfig,
    isCustomerNodeType,
    isSystemManagedNode,
    linkStatusOptions,
    linkTypeOptions,
    linksToFeatureCollection,
    nodeTypeLabel,
    nodeTypeOptions,
    nodesToFeatureCollection,
    parseGeometryText,
    prettyGeometry,
    routersToFeatureCollection,
    statusTone,
    systemManagedNodeSourceLabel,
    zonesToFeatureCollection,
    type LinkFieldConfig,
    type NMLink,
    type NMNode,
    type NMRouter,
    type NMZone,
  } from '$lib/components/network/networkMapUtils';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import 'maplibre-gl/dist/maplibre-gl.css';

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
  let syncingAssetNodes = $state(false);
  let locating = $state(false);
  let myLocationVisible = $state(false);
  let myLocationError = $state('');

  let nodesVisible = $state(true);
  let linksVisible = $state(true);
  let zonesVisible = $state(true);
  let routersVisible = $state(true);
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
  let routerRows = $state<NMRouter[]>([]);
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

  let refreshDebounce: ReturnType<typeof setTimeout> | null = null;
  let lastRequestId = 0;
  let myLocationMarker: import('maplibre-gl').Marker | null = null;
  let installationTargetMarker: import('maplibre-gl').Marker | null = null;
  let installationTargetCoord: [number, number] | null = null;
  let installationTargetResolved = false;
  let myLocationControlBtn: HTMLButtonElement | null = null;
  let viewModeControlBtn: HTMLButtonElement | null = null;
  let activeNodePopup: import('maplibre-gl').Popup | null = null;
  let activeDataAbortController: AbortController | null = null;
  let didInitialFitToMarkers = false;
  let lastAssetSyncAt = 0;
  const dataCache = new Map<
    string,
    NetworkMapCacheEntry
  >();
  const dataCacheTtlMs = 20_000;
  const dataCacheMaxEntries = 40;
  const assetSyncTtlMs = 45_000;
  const mapTilerKey = (import.meta.env.VITE_MAPTILER_KEY as string | undefined)?.trim();
  const hasHiResSatellite = Boolean(mapTilerKey);
  const standardMaxZoom = 19;
  const satelliteMaxZoom = hasHiResSatellite ? 21 : 18;

  const canManageTopology = $derived($can('manage', 'network_topology') || $can('manage', 'network_routers'));
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
    syncMyLocationControlUi();
  });

  $effect(() => {
    syncViewModeControlUi();
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

  function handleNodeLayerClick(e: any) {
    if (!map || !e.features?.[0] || !maplibre) return;
    const props = e.features[0].properties || {};
    const nodeId = String(props.id || '');
    if (linkPickMode) {
      handleLinkPickNode(nodeId);
      return;
    }
    openNodePopup({
      map,
      maplibre,
      feature: e.features[0],
      nodeRows,
      activePopup: activeNodePopup,
      setActivePopup: (popup) => (activeNodePopup = popup),
      onConnect: startConnectFromNode,
      onEdit: openEditNodeModal,
    });
  }

  function handleLinkLayerClick(e: any) {
    if (!map || !e.features?.[0] || !maplibre || linkPickMode) return;
    openLinkPopup({
      map,
      maplibre,
      feature: e.features[0],
      lngLat: e.lngLat,
      linkRows,
      onDelete: (linkId, linkName) => openDeleteConfirm('link', linkId, linkName),
    });
  }

  function handleRouterLayerClick(e: any) {
    if (!map || !e.features?.[0] || !maplibre) return;
    openRouterPopup({
      map,
      maplibre,
      feature: e.features[0],
      activePopup: activeNodePopup,
      setActivePopup: (popup) => (activeNodePopup = popup),
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

      map.addControl(new maplibre.NavigationControl({ showCompass: true, showZoom: true }), 'top-right');
      map.addControl(
        createMapTextButtonControl({
          className: 'nm-location-ctrl',
          onClick: () => {
            if (myLocationVisible) hideMyLocation();
            else void showMyLocation();
          },
          onMount: (btn) => {
            myLocationControlBtn = btn;
            syncMyLocationControlUi();
          },
          onUnmount: () => {
            myLocationControlBtn = null;
          },
        }),
        'top-right',
      );

      map.addControl(
        createMapTextButtonControl({
          className: 'nm-viewmode-ctrl',
          onClick: () => {
            viewMode = viewMode === 'standard' ? 'satellite' : 'standard';
          },
          onMount: (btn) => {
            viewModeControlBtn = btn;
            syncViewModeControlUi();
          },
          onUnmount: () => {
            viewModeControlBtn = null;
          },
        }),
        'top-right',
      );

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
        applyCachedMapData({
          cached,
          setRows: (rows) => {
            nodeRows = rows.nodeRows;
            linkRows = rows.linkRows;
            zoneRows = rows.zoneRows;
            routerRows = rows.routerRows;
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

      const result = await fetchNetworkMapData(params, abortController.signal);

      // Drop stale responses when user moves map quickly.
      if (requestId !== lastRequestId) return;
      if (abortController.signal.aborted) return;

      const rows = extractMapRows(result);

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

      applyFetchedMapData({
        result,
        setRows: (nextRows) => {
          nodeRows = nextRows.nodeRows;
          linkRows = nextRows.linkRows;
          zoneRows = nextRows.zoneRows;
          routerRows = nextRows.routerRows;
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

    locating = true;
    myLocationError = '';
    try {
      myLocationMarker = await showMyLocationMarker({
        map,
        maplibre,
        existingMarker: myLocationMarker,
      });
      myLocationVisible = true;
    } catch (e: any) {
      myLocationError = e?.message || 'Unable to get current location.';
      console.error(e);
    } finally {
      locating = false;
    }
  }

  function hideMyLocation() {
    hideMyLocationMarker(myLocationMarker);
    myLocationVisible = false;
    myLocationError = '';
  }

  function syncMyLocationControlUi() {
    const showLabel = $t('admin.network.map.location.show') || 'My Location';
    const hideLabel = $t('admin.network.map.location.hide') || 'Hide My Location';
    syncMyLocationControlButton({
      button: myLocationControlBtn,
      label: myLocationVisible ? hideLabel : showLabel,
      locating,
      mapUnavailable,
      myLocationVisible,
    });
  }

  function syncViewModeControlUi() {
    syncViewModeControlButton({
      button: viewModeControlBtn,
      isSatellite: viewMode === 'satellite',
    });
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
      toast.info(`Node ini tersinkron dari ${systemManagedNodeSourceLabel(row) || 'asset map'}. Ubah dari sumbernya.`);
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

  function onMapSearchSelect(event: CustomEvent<{ lat: number; lng: number; label: string }>) {
    if (!map) return;
    const { lat, lng } = event.detail;
    const currentZoom = Number.isFinite(map.getZoom()) ? map.getZoom() : 12;
    map.flyTo({
      center: [lng, lat],
      zoom: Math.max(currentZoom, 13),
      essential: true,
    });
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
    {nodeCount}
    {linkCount}
    {zoneCount}
    {q}
    {status}
    {kind}
    {nodesVisible}
    {linksVisible}
    {zonesVisible}
    {routersVisible}
    {customersVisible}
    {myLocationError}
    title={$t('admin.network.map.title') || 'Network Topology Map'}
    subtitle={$t('admin.network.map.subtitle') || 'Visualize nodes, links, and service zones in current viewport.'}
    labels={{
      backToInstallation: $t('admin.network.map.back_to_installation') || 'Back to Installation',
      backToNoc: $t('admin.network.map.back_to_noc') || 'Back to NOC',
      syncing: 'Syncing...',
      syncAssets: 'Sync Router & Customer Nodes',
      loading: $t('common.loading') || 'Loading...',
      refresh: $t('common.refresh') || 'Refresh',
      nodes: $t('admin.network.map.stats.nodes') || 'Nodes',
      links: $t('admin.network.map.stats.links') || 'Links',
      zones: $t('admin.network.map.stats.zones') || 'Zones',
      search: $t('admin.network.map.filters.search') || 'Search',
      searchPlaceholder: $t('admin.network.map.filters.search_placeholder') || 'Search node/link/zone...',
      status: $t('admin.network.map.filters.status') || 'Status',
      anyStatus: $t('admin.network.map.filters.any_status') || 'Any status',
      kind: $t('admin.network.map.filters.kind') || 'Type',
      anyKind: $t('admin.network.map.filters.any_kind') || 'Any type',
      apply: $t('common.apply') || 'Apply',
      reset: $t('common.reset') || 'Reset',
    }}
    onQChange={(value) => (q = value)}
    onStatusChange={(value) => (status = value)}
    onKindChange={(value) => (kind = value)}
    onApplyFilters={() => void onApplyFilters()}
    onResetFilters={onResetFilters}
    onSyncAssets={async () => {
      if (await syncTopologyAssets(true)) {
        invalidateMapDataCache();
      }
      await refreshMapData(true);
    }}
    onRefresh={() => void refreshMapData()}
    onNodesVisibleChange={(checked) => (nodesVisible = checked)}
    onLinksVisibleChange={(checked) => (linksVisible = checked)}
    onZonesVisibleChange={(checked) => (zonesVisible = checked)}
    onRoutersVisibleChange={(checked) => (routersVisible = checked)}
    onCustomersVisibleChange={(checked) => (customersVisible = checked)}
  />

  <MapCanvasShell
    bind:mapEl={mapEl}
    bind:viewMode={viewMode}
    on:searchselect={onMapSearchSelect}
    showViewSwitch={false}
    {loading}
    {mapUnavailable}
    {mapErrorMessage}
    mapUnavailableTitle="Map preview unavailable on this device"
    mapUnavailableSubtitle="WebGL context failed. Data is still loaded and counts are visible."
    height={compactMode ? 'min(76vh, 760px)' : 'min(62vh, 700px)'}
  >
    <svelte:fragment slot="overlay">
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
    </svelte:fragment>
  </MapCanvasShell>

  {#if !compactMode}
    <NetworkMapManager
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

  :global(.nm-viewmode-ctrl) {
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  :global(.nm-viewmode-ctrl:hover:not(:disabled)) {
    color: var(--text-primary);
  }

  :global(.nm-viewmode-ctrl.active) {
    color: #3f8cff;
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
  }

  @media (max-width: 560px) {
    .page-content :global(.network-page-actions) {
      grid-template-columns: 1fr;
    }
  }
</style>
