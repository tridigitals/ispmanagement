import { describe, expect, it, vi } from 'vitest';

import { fetchInterfaceMetricsRows } from './wallboardMetricsApi';

describe('fetchInterfaceMetricsRows', () => {
  it('returns an empty list without calling the API when the slot is missing', async () => {
    const fetchMetrics = vi.fn();

    const rows = await fetchInterfaceMetricsRows({
      slot: null,
      minLimit: 240,
      fetchMetrics,
    });

    expect(rows).toEqual([]);
    expect(fetchMetrics).not.toHaveBeenCalled();
  });
});
