import type { FeatureCollection } from 'geojson';
import { mikrotik } from '$lib/api/mikrotik';
import { networkMapping } from '$lib/api/networkMapping';
import type { PaginatedResponse } from '$lib/api/types';
import { toast } from '$lib/stores/toast';
import {
  customersToFeatureCollection,
  filterRoutersForOverlay,
  linksToFeatureCollection,
  nodesToFeatureCollection,
  routersToFeatureCollection,
  hasServiceMetadata,
  isCustomerNodeType,
  zonesToFeatureCollection,
  type NMLink,
  type NMNode,
  type NMRouter,
  type NMZone,
} from './networkMapUtils';

export type NetworkMapQueryParams = {
  q?: string;
  status?: string;
  kind?: string;
  bbox: string;
  page: number;
  per_page: number;
  include_legacy_ftth?: boolean;
};

export type NetworkMapCacheEntry = {
  at: number;
  nodes: PaginatedResponse<any>;
  links: PaginatedResponse<any>;
  zones: PaginatedResponse<any>;
  routers: NMRouter[];
};

export type NetworkMapFetchResult = {
  nodesRes: PaginatedResponse<any>;
  linksRes: PaginatedResponse<any>;
  zonesRes: PaginatedResponse<any>;
  routersRes: NMRouter[];
};

export type NetworkMapExtractedRows = {
  nodeRows: NMNode[];
  linkRows: NMLink[];
  zoneRows: NMZone[];
  routerRows: NMRouter[];
  customerRows: NMNode[];
  serviceRows: NMNode[];
  nodeCount: number;
  linkCount: number;
  zoneCount: number;
};

export const NETWORK_MAP_WORLD_BBOX = '-180,-85,180,85';

function buildDerivedRows(nodeRows: NMNode[], linkRows: NMLink[]) {
  return {
    customerRows: dedupeCustomerRows((nodeRows || []).filter((row) => isCustomerNodeType(row.node_type)), linkRows),
    serviceRows: (nodeRows || []).filter((row) => hasServiceMetadata(row)),
  };
}

function dedupeCustomerRows(customerRows: NMNode[], linkRows: NMLink[]) {
  const byKey = new Map<string, NMNode[]>();
  for (const row of customerRows || []) {
    const key = customerDedupKey(row);
    const bucket = byKey.get(key);
    if (bucket) {
      bucket.push(row);
    } else {
      byKey.set(key, [row]);
    }
  }

  return Array.from(byKey.values()).map((rows) =>
    rows
      .slice()
      .sort((a, b) => compareCustomerNodePriority(a, b, linkRows))[0],
  );
}

function customerDedupKey(row: NMNode) {
  const locationId = String(row.metadata?.location_id || '').trim();
  if (locationId) return `location:${locationId}`;
  const customerId = String(row.metadata?.customer_id || '').trim();
  if (customerId) return `customer:${customerId}`;
  return `node:${row.id}`;
}

function compareCustomerNodePriority(a: NMNode, b: NMNode, linkRows: NMLink[]) {
  const linkDelta = linkedEdgeCount(b.id, linkRows) - linkedEdgeCount(a.id, linkRows);
  if (linkDelta !== 0) return linkDelta;

  const stateDelta = customerStateRank(b) - customerStateRank(a);
  if (stateDelta !== 0) return stateDelta;

  return a.id.localeCompare(b.id);
}

function linkedEdgeCount(nodeId: string, linkRows: NMLink[]) {
  return (linkRows || []).reduce((count, row) => {
    return row.from_node_id === nodeId || row.to_node_id === nodeId ? count + 1 : count;
  }, 0);
}

function customerStateRank(row: NMNode) {
  const state = String(row.metadata?.pppoe_visual_state || '').trim().toLowerCase();
  if (state === 'connected') return 2;
  if (state === 'disconnected') return 1;
  return 0;
}

export function resolveNetworkMapFetchBbox({
  viewportBbox,
  initialExtentLoaded,
  hasActiveFilters,
}: {
  viewportBbox: string;
  initialExtentLoaded: boolean;
  hasActiveFilters: boolean;
}) {
  if (!initialExtentLoaded && !hasActiveFilters) return NETWORK_MAP_WORLD_BBOX;
  return viewportBbox;
}

export function buildMapDataCacheKey(params: NetworkMapQueryParams, _zoomSig: string) {
  return JSON.stringify({
    q: params.q || '',
    status: params.status || '',
    kind: params.kind || '',
    bbox: params.bbox,
  });
}

export function shouldFetchRouterOverlay({
  canReadRouterInventory,
  routersVisible,
}: {
  canReadRouterInventory: boolean;
  routersVisible: boolean;
}) {
  return canReadRouterInventory && routersVisible;
}

export function getCachedMapData(
  cache: Map<string, NetworkMapCacheEntry>,
  key: string,
  ttlMs: number,
) {
  const cached = cache.get(key);
  if (!cached) return undefined;
  if (Date.now() - cached.at > ttlMs) return undefined;
  return cached;
}

export function setCachedMapData(
  cache: Map<string, NetworkMapCacheEntry>,
  key: string,
  entry: Omit<NetworkMapCacheEntry, 'at'>,
  maxEntries: number,
) {
  cache.set(key, {
    at: Date.now(),
    ...entry,
  });
  if (cache.size > maxEntries) {
    const oldestKey = cache.keys().next().value as string | undefined;
    if (oldestKey) cache.delete(oldestKey);
  }
}

export async function fetchNetworkMapData(
  params: NetworkMapQueryParams,
  signal: AbortSignal,
  options: { includeRouters?: boolean } = {},
): Promise<NetworkMapFetchResult> {
  const includeRouters = options.includeRouters ?? true;
  const [nodesRes, linksRes, zonesRes, routersRes] = await Promise.all([
    networkMapping.nodes.list(
      {
        ...params,
        include_legacy_ftth: params.include_legacy_ftth ?? true,
      },
      { signal },
    ),
    networkMapping.links.list(params, { signal }),
    networkMapping.zones.list(params, { signal }),
    includeRouters ? mikrotik.routers.list() : Promise.resolve([]),
  ]);

  return {
    nodesRes,
    linksRes,
    zonesRes,
    routersRes: (routersRes || []) as NMRouter[],
  };
}

export async function fetchAllPaginatedRows<T>(
  fetchPage: (page: number) => Promise<{ data?: T[]; total?: number; page?: number; per_page?: number }>,
) {
  const first = await fetchPage(1);
  const rows = [...(first.data || [])];
  const total = Math.max(Number(first.total || rows.length), rows.length);
  const perPage = Math.max(Number(first.per_page || rows.length || 1), 1);
  const maxPage = Math.max(Math.ceil(total / perPage), 1);

  for (let page = 2; page <= maxPage && rows.length < total; page += 1) {
    const next = await fetchPage(page);
    rows.push(...(next.data || []));
  }

  return rows;
}

export function getTopologySyncStrategy({
  canManageTopology,
  syncingAssetNodes,
  manual,
  lastAssetSyncAt,
  assetSyncTtlMs,
  now = Date.now(),
}: {
  canManageTopology: boolean;
  syncingAssetNodes: boolean;
  manual: boolean;
  lastAssetSyncAt: number;
  assetSyncTtlMs: number;
  now?: number;
}): { shouldSync: boolean; shouldBlockRefresh: boolean } {
  if (!canManageTopology || syncingAssetNodes) {
    return { shouldSync: false, shouldBlockRefresh: false };
  }

  if (!manual && now - lastAssetSyncAt < assetSyncTtlMs) {
    return { shouldSync: false, shouldBlockRefresh: false };
  }

  return {
    shouldSync: true,
    shouldBlockRefresh: manual,
  };
}

export async function syncTopologyAssetsIfNeeded({
  canManageTopology,
  syncingAssetNodes,
  manual,
  lastAssetSyncAt,
  assetSyncTtlMs,
}: {
  canManageTopology: boolean;
  syncingAssetNodes: boolean;
  manual: boolean;
  lastAssetSyncAt: number;
  assetSyncTtlMs: number;
}): Promise<{ changed: boolean; lastSyncedAt: number }> {
  const strategy = getTopologySyncStrategy({
    canManageTopology,
    syncingAssetNodes,
    manual,
    lastAssetSyncAt,
    assetSyncTtlMs,
  });
  if (!strategy.shouldSync) {
    return { changed: false, lastSyncedAt: lastAssetSyncAt };
  }

  try {
    const result = await networkMapping.assets.sync();
    const lastSyncedAt = Date.now();
    if (manual) {
      toast.success(
        `Topology sync selesai. Router: ${result.router_nodes_created + result.router_nodes_updated}, Asset: ${result.asset_nodes_created + result.asset_nodes_updated}, Customer: ${result.customer_nodes_created + result.customer_nodes_updated}.`,
      );
    }
    return {
      changed: result.total_nodes_touched > 0,
      lastSyncedAt,
    };
  } catch (e: any) {
    if (manual) {
      toast.error(e?.message || 'Failed to sync topology assets');
    } else {
      console.error(e);
    }
    return { changed: false, lastSyncedAt: lastAssetSyncAt };
  }
}

export function extractMapRows(result: NetworkMapFetchResult): NetworkMapExtractedRows {
  const nodeRows = (result.nodesRes.data || []) as NMNode[];
  const linkRows = (result.linksRes.data || []) as NMLink[];
  const zoneRows = (result.zonesRes.data || []) as NMZone[];
  const derivedRows = buildDerivedRows(nodeRows, linkRows);

  return {
    nodeRows,
    linkRows,
    zoneRows,
    routerRows: result.routersRes,
    nodeCount: result.nodesRes.total || result.nodesRes.data?.length || 0,
    linkCount: result.linksRes.total || result.linksRes.data?.length || 0,
    zoneCount: result.zonesRes.total || result.zonesRes.data?.length || 0,
    ...derivedRows,
  };
}

type NetworkMapRowViewState = NetworkMapExtractedRows;

export function applyCachedMapData(args: {
  cached: NetworkMapCacheEntry;
  setRows: (rows: NetworkMapRowViewState) => void;
  setSourceData: (sourceId: string, data: FeatureCollection) => void;
  sourceIds: {
    nodes: string;
    customers: string;
    links: string;
    zones: string;
    routers: string;
  };
  fitToMarkers: (nodeRows: NMNode[], routerRows: NMRouter[]) => void;
}) {
  const nodeRows = (args.cached.nodes.data || []) as NMNode[];
  const linkRows = (args.cached.links.data || []) as NMLink[];
  const zoneRows = (args.cached.zones.data || []) as NMZone[];
  const routerRows = (args.cached.routers || []) as NMRouter[];
  const derivedRows = buildDerivedRows(nodeRows, linkRows);
  const routerOverlayRows = filterRoutersForOverlay(routerRows, nodeRows);

  args.setRows({
    nodeRows,
    linkRows,
    zoneRows,
    routerRows,
    nodeCount: args.cached.nodes.total || args.cached.nodes.data?.length || 0,
    linkCount: args.cached.links.total || args.cached.links.data?.length || 0,
    zoneCount: args.cached.zones.total || args.cached.zones.data?.length || 0,
    ...derivedRows,
  });

  args.setSourceData(args.sourceIds.nodes, nodesToFeatureCollection(nodeRows));
  args.setSourceData(args.sourceIds.customers, customersToFeatureCollection(derivedRows.customerRows));
  args.setSourceData(args.sourceIds.links, linksToFeatureCollection(linkRows));
  args.setSourceData(args.sourceIds.zones, zonesToFeatureCollection(zoneRows));
  args.setSourceData(args.sourceIds.routers, routersToFeatureCollection(routerOverlayRows));
  args.fitToMarkers(nodeRows, routerOverlayRows);
}

export function applyFetchedMapData(args: {
  result: NetworkMapFetchResult;
  setRows: (rows: NetworkMapRowViewState) => void;
  setSourceData: (sourceId: string, data: FeatureCollection) => void;
  sourceIds: {
    nodes: string;
    customers: string;
    links: string;
    zones: string;
    routers: string;
  };
  fitToMarkers: (nodeRows: NMNode[], routerRows: NMRouter[]) => void;
}) {
  const rows = extractMapRows(args.result);
  const routerOverlayRows = filterRoutersForOverlay(rows.routerRows, rows.nodeRows);

  args.setRows(rows);
  args.setSourceData(args.sourceIds.nodes, nodesToFeatureCollection(rows.nodeRows));
  args.setSourceData(args.sourceIds.customers, customersToFeatureCollection(rows.customerRows));
  args.setSourceData(args.sourceIds.links, linksToFeatureCollection(rows.linkRows));
  args.setSourceData(args.sourceIds.zones, zonesToFeatureCollection(rows.zoneRows));
  args.setSourceData(args.sourceIds.routers, routersToFeatureCollection(routerOverlayRows));
  args.fitToMarkers(rows.nodeRows, routerOverlayRows);
}
