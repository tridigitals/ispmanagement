import type { FeatureCollection } from 'geojson';
import { api, type PaginatedResponse } from '$lib/api/client';
import { toast } from '$lib/stores/toast';
import {
  customersToFeatureCollection,
  filterRoutersForOverlay,
  linksToFeatureCollection,
  nodesToFeatureCollection,
  routersToFeatureCollection,
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

export function buildMapDataCacheKey(params: NetworkMapQueryParams, zoomSig: string) {
  return JSON.stringify({
    q: params.q || '',
    status: params.status || '',
    kind: params.kind || '',
    bbox: params.bbox,
    zoom: zoomSig,
  });
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
): Promise<NetworkMapFetchResult> {
  const [nodesRes, linksRes, zonesRes, routersRes] = await Promise.all([
    api.networkMapping.nodes.list(params, { signal }),
    api.networkMapping.links.list(params, { signal }),
    api.networkMapping.zones.list(params, { signal }),
    api.mikrotik.routers.list(),
  ]);

  return {
    nodesRes,
    linksRes,
    zonesRes,
    routersRes: (routersRes || []) as NMRouter[],
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
  if (!canManageTopology || syncingAssetNodes) {
    return { changed: false, lastSyncedAt: lastAssetSyncAt };
  }

  const now = Date.now();
  if (!manual && now - lastAssetSyncAt < assetSyncTtlMs) {
    return { changed: false, lastSyncedAt: lastAssetSyncAt };
  }

  try {
    const result = await api.networkMapping.assets.sync();
    const lastSyncedAt = Date.now();
    if (manual) {
      toast.success(
        `Topology sync selesai. Router: ${result.router_nodes_created + result.router_nodes_updated}, Customer: ${result.customer_nodes_created + result.customer_nodes_updated}.`,
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

export function extractMapRows(result: NetworkMapFetchResult): {
  nodeRows: NMNode[];
  linkRows: NMLink[];
  zoneRows: NMZone[];
  routerRows: NMRouter[];
  nodeCount: number;
  linkCount: number;
  zoneCount: number;
} {
  return {
    nodeRows: (result.nodesRes.data || []) as NMNode[],
    linkRows: (result.linksRes.data || []) as NMLink[],
    zoneRows: (result.zonesRes.data || []) as NMZone[],
    routerRows: result.routersRes,
    nodeCount: result.nodesRes.total || result.nodesRes.data?.length || 0,
    linkCount: result.linksRes.total || result.linksRes.data?.length || 0,
    zoneCount: result.zonesRes.total || result.zonesRes.data?.length || 0,
  };
}

export function applyCachedMapData(args: {
  cached: NetworkMapCacheEntry;
  setRows: (rows: {
    nodeRows: NMNode[];
    linkRows: NMLink[];
    zoneRows: NMZone[];
    routerRows: NMRouter[];
    nodeCount: number;
    linkCount: number;
    zoneCount: number;
  }) => void;
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
  const routerOverlayRows = filterRoutersForOverlay(routerRows, nodeRows);

  args.setRows({
    nodeRows,
    linkRows,
    zoneRows,
    routerRows,
    nodeCount: args.cached.nodes.total || args.cached.nodes.data?.length || 0,
    linkCount: args.cached.links.total || args.cached.links.data?.length || 0,
    zoneCount: args.cached.zones.total || args.cached.zones.data?.length || 0,
  });

  args.setSourceData(args.sourceIds.nodes, nodesToFeatureCollection(nodeRows));
  args.setSourceData(args.sourceIds.customers, customersToFeatureCollection(nodeRows));
  args.setSourceData(args.sourceIds.links, linksToFeatureCollection(linkRows));
  args.setSourceData(args.sourceIds.zones, zonesToFeatureCollection(zoneRows));
  args.setSourceData(args.sourceIds.routers, routersToFeatureCollection(routerOverlayRows));
  args.fitToMarkers(nodeRows, routerOverlayRows);
}

export function applyFetchedMapData(args: {
  result: NetworkMapFetchResult;
  setRows: (rows: {
    nodeRows: NMNode[];
    linkRows: NMLink[];
    zoneRows: NMZone[];
    routerRows: NMRouter[];
    nodeCount: number;
    linkCount: number;
    zoneCount: number;
  }) => void;
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
  args.setSourceData(args.sourceIds.customers, customersToFeatureCollection(rows.nodeRows));
  args.setSourceData(args.sourceIds.links, linksToFeatureCollection(rows.linkRows));
  args.setSourceData(args.sourceIds.zones, zonesToFeatureCollection(rows.zoneRows));
  args.setSourceData(args.sourceIds.routers, routersToFeatureCollection(routerOverlayRows));
  args.fitToMarkers(rows.nodeRows, routerOverlayRows);
}
