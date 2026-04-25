import { beforeEach, describe, expect, it, vi } from 'vitest';

const { listNodes, listLinks, listZones, listRouters } = vi.hoisted(() => ({
  listNodes: vi.fn(),
  listLinks: vi.fn(),
  listZones: vi.fn(),
  listRouters: vi.fn(),
}));

vi.mock('$lib/api/networkMapping', () => ({
  networkMapping: {
    nodes: { list: listNodes },
    links: { list: listLinks },
    zones: { list: listZones },
  },
}));

vi.mock('$lib/api/mikrotik', () => ({
  mikrotik: {
    routers: { list: listRouters },
  },
}));

import {
  buildMapDataCacheKey,
  extractMapRows,
  fetchNetworkMapData,
  NETWORK_MAP_WORLD_BBOX,
  getTopologySyncStrategy,
  resolveNetworkMapFetchBbox,
  shouldFetchRouterOverlay,
} from './networkMapData';

describe('fetchNetworkMapData', () => {
  const params = {
    q: undefined,
    status: undefined,
    kind: undefined,
    bbox: '1,2,3,4',
    page: 1,
    per_page: 1000,
  };

  beforeEach(() => {
    listNodes.mockReset();
    listLinks.mockReset();
    listZones.mockReset();
    listRouters.mockReset();

    listNodes.mockResolvedValue({ data: [], total: 0 });
    listLinks.mockResolvedValue({ data: [], total: 0 });
    listZones.mockResolvedValue({ data: [], total: 0 });
    listRouters.mockResolvedValue([{ id: 'router-1', name: 'Router 1' }]);
  });

  it('skips router inventory fetch when router overlay access is unavailable', async () => {
    const result = await fetchNetworkMapData(params, new AbortController().signal, {
      includeRouters: false,
    });

    expect(listNodes).toHaveBeenCalledOnce();
    expect(listLinks).toHaveBeenCalledOnce();
    expect(listZones).toHaveBeenCalledOnce();
    expect(listRouters).not.toHaveBeenCalled();
    expect(result.routersRes).toEqual([]);
  });

  it('loads router inventory when router overlay access is available', async () => {
    const result = await fetchNetworkMapData(params, new AbortController().signal, {
      includeRouters: true,
    });

    expect(listRouters).toHaveBeenCalledOnce();
    expect(result.routersRes).toEqual([{ id: 'router-1', name: 'Router 1' }]);
  });

  it('derives customer and service placeholder rows without changing fetch rows', () => {
    const result = extractMapRows({
      nodesRes: {
        data: [
          {
            id: 'customer-node',
            name: 'Customer Premise',
            node_type: 'customer_premise',
            status: 'active',
            lat: -6.2,
            lng: 106.8,
          },
          {
            id: 'service-node',
            name: 'Managed Service',
            node_type: 'switch',
            status: 'active',
            lat: -6.21,
            lng: 106.81,
            metadata: { service_id: 'svc-1', service_name: 'Managed Service' },
          },
        ],
        total: 2,
        page: 1,
        per_page: 1000,
      },
      linksRes: { data: [], total: 0, page: 1, per_page: 1000 },
      zonesRes: { data: [], total: 0, page: 1, per_page: 1000 },
      routersRes: [{ id: 'router-1', name: 'Router 1' } as any],
    });

    expect(result.nodeRows).toHaveLength(2);
    expect(result.customerRows).toEqual([
      expect.objectContaining({ id: 'customer-node', node_type: 'customer_premise' }),
    ]);
    expect(result.serviceRows).toEqual([
      expect.objectContaining({
        id: 'service-node',
        metadata: expect.objectContaining({ service_id: 'svc-1' }),
      }),
    ]);
    expect(result.routerRows).toEqual([{ id: 'router-1', name: 'Router 1' }]);
  });
});

describe('resolveNetworkMapFetchBbox', () => {
  it('uses world extent for the first unfiltered load so initial fit can see all markers', () => {
    expect(
      resolveNetworkMapFetchBbox({
        viewportBbox: '106,-7,107,-6',
        initialExtentLoaded: false,
        hasActiveFilters: false,
      }),
    ).toBe(NETWORK_MAP_WORLD_BBOX);
  });

  it('uses viewport extent after the initial load or when filters are active', () => {
    expect(
      resolveNetworkMapFetchBbox({
        viewportBbox: '106,-7,107,-6',
        initialExtentLoaded: true,
        hasActiveFilters: false,
      }),
    ).toBe('106,-7,107,-6');

    expect(
      resolveNetworkMapFetchBbox({
        viewportBbox: '106,-7,107,-6',
        initialExtentLoaded: false,
        hasActiveFilters: true,
      }),
    ).toBe('106,-7,107,-6');
  });
});

describe('buildMapDataCacheKey', () => {
  it('keeps cache keys stable across zoom changes because backend data is bbox based', () => {
    const params = {
      q: undefined,
      status: undefined,
      kind: undefined,
      bbox: '106,-7,107,-6',
      page: 1,
      per_page: 1000,
    };

    expect(buildMapDataCacheKey(params, '10.00')).toBe(buildMapDataCacheKey(params, '13.00'));
  });
});

describe('shouldFetchRouterOverlay', () => {
  it('fetches router inventory only when the user can read it and the router layer is visible', () => {
    expect(
      shouldFetchRouterOverlay({
        canReadRouterInventory: true,
        routersVisible: true,
      }),
    ).toBe(true);

    expect(
      shouldFetchRouterOverlay({
        canReadRouterInventory: true,
        routersVisible: false,
      }),
    ).toBe(false);

    expect(
      shouldFetchRouterOverlay({
        canReadRouterInventory: false,
        routersVisible: true,
      }),
    ).toBe(false);
  });
});

describe('getTopologySyncStrategy', () => {
  it('runs stale automatic sync in background so initial map fetch is not blocked', () => {
    expect(
      getTopologySyncStrategy({
        canManageTopology: true,
        syncingAssetNodes: false,
        manual: false,
        lastAssetSyncAt: 0,
        assetSyncTtlMs: 45_000,
        now: 45_001,
      }),
    ).toEqual({
      shouldSync: true,
      shouldBlockRefresh: false,
    });
  });

  it('keeps manual sync blocking so explicit sync action stays deterministic', () => {
    expect(
      getTopologySyncStrategy({
        canManageTopology: true,
        syncingAssetNodes: false,
        manual: true,
        lastAssetSyncAt: 0,
        assetSyncTtlMs: 45_000,
        now: 45_001,
      }),
    ).toEqual({
      shouldSync: true,
      shouldBlockRefresh: true,
    });
  });

  it('skips sync entirely while still inside TTL', () => {
    expect(
      getTopologySyncStrategy({
        canManageTopology: true,
        syncingAssetNodes: false,
        manual: false,
        lastAssetSyncAt: 20_000,
        assetSyncTtlMs: 45_000,
        now: 40_000,
      }),
    ).toEqual({
      shouldSync: false,
      shouldBlockRefresh: false,
    });
  });
});
