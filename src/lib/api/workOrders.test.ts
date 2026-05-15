import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('workOrders api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('completes installation work order with asset binding payload', async () => {
    safeInvoke.mockResolvedValue({ id: 'wo-1', status: 'completed' });

    const { workOrders } = await import('./workOrders');
    await workOrders.complete('wo-1', {
      notes: 'ONT installed and tested',
      terminal_asset_id: 'asset-ont-1',
      parent_asset_id: 'asset-odp-1',
    });

    expect(safeInvoke).toHaveBeenCalledWith('complete_installation_work_order', {
      token: 'token-123',
      id: 'wo-1',
      notes: 'ONT installed and tested',
      terminal_asset_id: 'asset-ont-1',
      parent_asset_id: 'asset-odp-1',
    });
  });
});
