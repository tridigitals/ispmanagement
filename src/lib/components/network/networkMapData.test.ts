import { beforeEach, describe, expect, it, vi } from 'vitest';

const { listNodes, listLinks, listZones, listRouters } = vi.hoisted(() => ({
  listNodes: vi.fn(),
  listLinks: vi.fn(),
  listZones: vi.fn(),
  listRouters: vi.fn(),
}));

vi.mock('$lib/api/client', () => ({
  api: {
    networkMapping: {
      nodes: { list: listNodes },
      links: { list: listLinks },
      zones: { list: listZones },
    },
    mikrotik: {
      routers: { list: listRouters },
    },
  },
}));

import { extractMapRows, fetchNetworkMapData } from './networkMapData';

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
