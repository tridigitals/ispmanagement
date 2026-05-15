import { describe, expect, it } from 'vitest';

import {
  buildNetworkAssetStats,
  buildNetworkAssetSavePayload,
  filterNetworkAssets,
  normalizeNetworkAssetSearch,
} from './networkAssetsPageState';

const rows = [
  {
    id: '1',
    tenant_id: 'tenant-1',
    asset_group: 'access_fiber',
    asset_type: 'odp',
    name: 'ODP-Alpha',
    code: 'ODP-01',
    vendor: null,
    model: null,
    serial_number: 'SER-001',
    status: 'available',
    customer_id: null,
    location_id: null,
    work_order_id: null,
    parent_asset_id: null,
    latitude: null,
    longitude: null,
    notes: null,
    metadata: {},
    created_at: '2026-05-10T00:00:00Z',
    updated_at: '2026-05-10T10:00:00Z',
    customer_name: null,
    location_label: null,
    work_order_status: null,
    parent_asset_name: null,
  },
  {
    id: '2',
    tenant_id: 'tenant-1',
    asset_group: 'access_fiber',
    asset_type: 'ont',
    name: 'ONT ZTE',
    code: 'ONT-11',
    vendor: null,
    model: null,
    serial_number: 'SN-123',
    status: 'installed',
    customer_id: 'cust-1',
    location_id: 'loc-1',
    work_order_id: null,
    parent_asset_id: null,
    latitude: null,
    longitude: null,
    notes: null,
    metadata: {},
    created_at: '2026-05-10T00:00:00Z',
    updated_at: '2026-05-11T10:00:00Z',
    customer_name: 'Andi',
    location_label: 'Rumah',
    work_order_status: null,
    parent_asset_name: null,
  },
] as const;

describe('network assets page state', () => {
  it('normalizes search values for case-insensitive filtering', () => {
    expect(normalizeNetworkAssetSearch('  ONT  ')).toBe('ont');
  });

  it('filters by asset type, status, and search text', () => {
    expect(filterNetworkAssets([...rows], { assetType: 'ont', status: 'installed' })).toEqual([
      rows[1],
    ]);
    expect(filterNetworkAssets([...rows], { q: 'ser-001' })).toEqual([rows[0]]);
  });

  it('builds top-level stats from the current asset collection', () => {
    expect(buildNetworkAssetStats([...rows])).toEqual({
      total: 2,
      installed: 1,
      available: 1,
      faulty: 0,
    });
  });

  it('builds a create payload without operational relations from the manual form', () => {
    expect(
      buildNetworkAssetSavePayload({
        draft: {
          asset_type: 'odp',
          name: 'ODP Manual',
          code: 'ODP-100',
          vendor: '',
          model: '',
          serial_number: '',
          status: 'available',
          customer_id: '',
          location_id: '',
          work_order_id: '',
          parent_asset_id: '',
          latitude: '-7.2647003',
          longitude: '110.3861725',
          notes: '',
        },
        metadata: { total_port_capacity: '8' },
      }),
    ).toMatchObject({
      asset_type: 'odp',
      name: 'ODP Manual',
      customer_id: null,
      location_id: null,
      work_order_id: null,
      parent_asset_id: null,
      latitude: -7.2647003,
      longitude: 110.3861725,
    });
  });

  it('preserves existing operational relations when editing an asset from the manual form', () => {
    expect(
      buildNetworkAssetSavePayload({
        draft: {
          asset_type: 'ont',
          name: 'ONT Edited',
          code: 'ONT-11',
          vendor: '',
          model: '',
          serial_number: 'SN-123',
          status: 'reserved',
          customer_id: 'ignored-in-ui',
          location_id: 'ignored-in-ui',
          work_order_id: 'ignored-in-ui',
          parent_asset_id: 'ignored-in-ui',
          latitude: '',
          longitude: '',
          notes: '',
        },
        metadata: {},
        existingRelations: {
          customer_id: 'cust-1',
          location_id: 'loc-1',
          work_order_id: 'wo-1',
          parent_asset_id: 'odp-1',
        },
      }),
    ).toMatchObject({
      customer_id: 'cust-1',
      location_id: 'loc-1',
      work_order_id: 'wo-1',
      parent_asset_id: 'odp-1',
      latitude: null,
      longitude: null,
    });
  });
});
