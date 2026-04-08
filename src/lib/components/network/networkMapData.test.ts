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

import { fetchNetworkMapData } from './networkMapData';

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
});
