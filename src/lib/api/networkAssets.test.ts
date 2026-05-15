import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('networkAssets api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('lists FTTH assets with tenant-scoped filters', async () => {
    safeInvoke.mockResolvedValue({ data: [], total: 0, page: 1, per_page: 25 });

    const { networkAssets } = await import('./networkAssets');
    await networkAssets.list({
      q: 'odp',
      asset_type: 'odp',
      status: 'available',
      customer_id: 'cust-1',
      location_id: 'loc-1',
      page: 2,
      per_page: 25,
    });

    expect(safeInvoke).toHaveBeenCalledWith('list_network_assets', {
      token: 'token-123',
      q: 'odp',
      asset_type: 'odp',
      status: 'available',
      customer_id: 'cust-1',
      location_id: 'loc-1',
      page: 2,
      per_page: 25,
    });
  });

  it('creates FTTH assets with stable payload fields', async () => {
    safeInvoke.mockResolvedValue({ id: 'asset-1' });

    const { networkAssets } = await import('./networkAssets');
    await networkAssets.create({
      asset_type: 'ont',
      name: 'ONT ZTE F670L',
      code: 'ONT-001',
      vendor: 'ZTE',
      model: 'F670L',
      serial_number: 'SN-123',
      status: 'available',
      customer_id: null,
      location_id: null,
      work_order_id: null,
      parent_asset_id: null,
      latitude: -6.21462,
      longitude: 106.84513,
      notes: 'Warehouse ready',
      metadata: { pon_brand: 'zte' },
    });

    expect(safeInvoke).toHaveBeenCalledWith('create_network_asset', {
      token: 'token-123',
      asset_type: 'ont',
      name: 'ONT ZTE F670L',
      code: 'ONT-001',
      vendor: 'ZTE',
      model: 'F670L',
      serial_number: 'SN-123',
      status: 'available',
      customer_id: null,
      location_id: null,
      work_order_id: null,
      parent_asset_id: null,
      latitude: -6.21462,
      longitude: 106.84513,
      notes: 'Warehouse ready',
      metadata: { pon_brand: 'zte' },
    });
  });

  it('updates FTTH assets with id plus partial payload', async () => {
    safeInvoke.mockResolvedValue({ id: 'asset-1', status: 'installed' });

    const { networkAssets } = await import('./networkAssets');
    await networkAssets.update('asset-1', {
      status: 'installed',
      customer_id: 'cust-1',
      location_id: 'loc-1',
      latitude: -6.2,
      longitude: 106.8,
    });

    expect(safeInvoke).toHaveBeenCalledWith('update_network_asset', {
      token: 'token-123',
      id: 'asset-1',
      status: 'installed',
      customer_id: 'cust-1',
      location_id: 'loc-1',
      latitude: -6.2,
      longitude: 106.8,
    });
  });

  it('assigns asset relations through dedicated command helpers', async () => {
    safeInvoke.mockResolvedValue({ id: 'asset-1', customer_id: 'cust-1' });

    const { networkAssets } = await import('./networkAssets');
    await networkAssets.assignCustomer('asset-1', 'cust-1');
    await networkAssets.assignLocation('asset-1', 'loc-1');
    await networkAssets.assignWorkOrder('asset-1', 'wo-1');
    await networkAssets.linkParentAsset('asset-1', 'parent-1');

    expect(safeInvoke).toHaveBeenNthCalledWith(1, 'assign_network_asset_customer', {
      token: 'token-123',
      id: 'asset-1',
      customer_id: 'cust-1',
    });
    expect(safeInvoke).toHaveBeenNthCalledWith(2, 'assign_network_asset_location', {
      token: 'token-123',
      id: 'asset-1',
      location_id: 'loc-1',
    });
    expect(safeInvoke).toHaveBeenNthCalledWith(3, 'assign_network_asset_work_order', {
      token: 'token-123',
      id: 'asset-1',
      work_order_id: 'wo-1',
    });
    expect(safeInvoke).toHaveBeenNthCalledWith(4, 'link_network_asset_parent', {
      token: 'token-123',
      id: 'asset-1',
      parent_asset_id: 'parent-1',
    });
  });

  it('lists customer assets through a dedicated customer lookup', async () => {
    safeInvoke.mockResolvedValue([]);

    const { networkAssets } = await import('./networkAssets');
    await networkAssets.listCustomerAssets('cust-1');

    expect(safeInvoke).toHaveBeenCalledWith('list_customer_network_assets', {
      token: 'token-123',
      customer_id: 'cust-1',
    });
  });
});
