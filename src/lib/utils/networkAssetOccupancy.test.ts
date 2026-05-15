import { describe, expect, it } from 'vitest';

import type { NetworkAssetListItem } from '$lib/api/types';

import {
  buildNetworkAssetOccupancyLabel,
  getNetworkAssetPortOccupancy,
  getNetworkAssetPortOccupancySummary,
} from './networkAssetOccupancy';

function asset(overrides: Partial<NetworkAssetListItem>): NetworkAssetListItem {
  return {
    id: 'asset-1',
    tenant_id: 'tenant-1',
    asset_group: 'access_fiber',
    asset_type: 'odp',
    name: 'ODP-1',
    code: null,
    vendor: null,
    model: null,
    serial_number: null,
    status: 'available',
    customer_id: null,
    location_id: null,
    work_order_id: null,
    parent_asset_id: null,
    latitude: null,
    longitude: null,
    notes: null,
    metadata: {},
    created_at: '2026-05-12T00:00:00Z',
    updated_at: '2026-05-12T00:00:00Z',
    customer_name: null,
    location_label: null,
    work_order_status: null,
    parent_asset_name: null,
    ...overrides,
  };
}

describe('networkAssetOccupancy', () => {
  it('computes ODP occupancy from linked terminal assets', () => {
    expect(
      getNetworkAssetPortOccupancy(
        asset({
          id: 'odp-1',
          metadata: {
            total_port_capacity: '8',
          },
        }),
        [
          asset({
            id: 'ont-1',
            asset_type: 'ont',
            parent_asset_id: 'odp-1',
            status: 'installed',
          }),
          asset({
            id: 'onu-1',
            asset_type: 'onu',
            parent_asset_id: 'odp-1',
            status: 'available',
          }),
          asset({
            id: 'ont-retired',
            asset_type: 'ont',
            parent_asset_id: 'odp-1',
            status: 'retired',
          }),
          asset({
            id: 'switch-1',
            asset_type: 'switch',
            parent_asset_id: 'odp-1',
            status: 'available',
          }),
        ],
      ),
    ).toEqual({
      total: 8,
      used: 2,
      available: 6,
      state: 'partial',
    });
  });

  it('returns null for non-ODP assets or missing capacity', () => {
    expect(
      getNetworkAssetPortOccupancy(asset({ asset_type: 'ont' }), [asset({ id: 'odp-1' })]),
    ).toBeNull();

    expect(getNetworkAssetPortOccupancy(asset({ id: 'odp-1', metadata: {} }), [])).toBeNull();
  });

  it('builds readable occupancy summary and compact label', () => {
    const odp = asset({
      id: 'odp-1',
      metadata: {
        total_port_capacity: '16',
      },
    });
    const allAssets = [
      odp,
      asset({ id: 'ont-1', asset_type: 'ont', parent_asset_id: 'odp-1', status: 'installed' }),
      asset({ id: 'ont-2', asset_type: 'ont', parent_asset_id: 'odp-1', status: 'installed' }),
    ];

    expect(getNetworkAssetPortOccupancySummary(odp, allAssets)).toEqual([
      'Port Capacity: 16',
      'Ports Used: 2',
      'Ports Available: 14',
    ]);
    expect(buildNetworkAssetOccupancyLabel(odp, allAssets)).toBe('2/16 used');
  });

  it('counts hybrid ODP usage from terminal assets and direct customer attachments without double counting', () => {
    const odp = asset({
      id: 'odp-1',
      metadata: {
        total_port_capacity: '8',
      },
    });

    const allAssets = [
      odp,
      asset({
        id: 'ont-1',
        asset_type: 'ont',
        parent_asset_id: 'odp-1',
        customer_id: 'cust-1',
        location_id: 'loc-1',
        status: 'installed',
      }),
      asset({
        id: 'service-drop-1',
        asset_type: 'media_converter',
        parent_asset_id: 'odp-1',
        customer_id: 'cust-1',
        location_id: 'loc-1',
        status: 'installed',
      }),
      asset({
        id: 'service-drop-2',
        asset_type: 'media_converter',
        parent_asset_id: 'odp-1',
        customer_id: 'cust-2',
        location_id: 'loc-2',
        status: 'reserved',
      }),
      asset({
        id: 'service-drop-retired',
        asset_type: 'media_converter',
        parent_asset_id: 'odp-1',
        customer_id: 'cust-3',
        location_id: 'loc-3',
        status: 'retired',
      }),
    ];

    expect(getNetworkAssetPortOccupancy(odp, allAssets)).toEqual({
      total: 8,
      used: 2,
      available: 6,
      state: 'partial',
    });
  });
});
