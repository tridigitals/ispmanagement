import { describe, expect, it } from 'vitest';

import type { NetworkAssetListItem } from '$lib/api/types';

import {
  buildNetworkAssetOccupancyLabel,
  getNetworkAssetPortOccupancy,
  getNetworkAssetPortOccupancySummary,
} from './networkAssetOccupancy';
import type { NMLink, NMNode, NMRouter } from '$lib/components/network/networkMapUtils';

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

function node(overrides: Partial<NMNode>): NMNode {
  return {
    id: 'node-1',
    name: 'Node 1',
    node_type: 'customer_premise',
    status: 'active',
    lat: -7.2,
    lng: 110.3,
    metadata: {},
    ...overrides,
  };
}

function link(overrides: Partial<NMLink>): NMLink {
  return {
    id: 'link-1',
    name: 'Link 1',
    link_type: 'fiber',
    status: 'up',
    from_node_id: 'node-a',
    to_node_id: 'node-b',
    geometry: {
      type: 'LineString',
      coordinates: [
        [110.3, -7.2],
        [110.31, -7.21],
      ],
    },
    ...overrides,
  };
}

function router(overrides: Partial<NMRouter>): NMRouter {
  return {
    id: 'router-1',
    name: 'Router 1',
    host: '10.10.10.1',
    port: 8728,
    is_online: true,
    enabled: true,
    identity: 'RTR-1',
    ros_version: '7.18.2',
    latency_ms: 3,
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

  it('counts saved customer topology links as used ODP ports without double counting linked ONT endpoints', () => {
    const odp = asset({
      id: 'odp-1',
      metadata: {
        total_port_capacity: '8',
      },
    });

    expect(
      getNetworkAssetPortOccupancy(
        odp,
        [
          odp,
          asset({
            id: 'ont-1',
            asset_type: 'ont',
            parent_asset_id: 'odp-1',
            customer_id: 'cust-1',
            location_id: 'loc-1',
            status: 'installed',
          }),
        ],
        {
          nodeRows: [
            node({
              id: 'odp-node-1',
              node_type: 'odp',
              metadata: {
                asset_source: 'network_asset',
                asset_type: 'odp',
                asset_id: 'odp-1',
              },
            }),
            node({
              id: 'customer-node-1',
              node_type: 'customer_premise',
              status: 'active',
              metadata: {
                asset_source: 'customer_location',
                asset_type: 'customer_location',
                customer_id: 'cust-1',
                location_id: 'loc-1',
              },
            }),
            node({
              id: 'customer-node-2',
              node_type: 'customer_premise',
              status: 'active',
              metadata: {
                asset_source: 'customer_location',
                asset_type: 'customer_location',
                customer_id: 'cust-2',
                location_id: 'loc-2',
              },
            }),
            node({
              id: 'customer-node-3',
              node_type: 'customer_premise',
              status: 'inactive',
              metadata: {
                asset_source: 'customer_location',
                asset_type: 'customer_location',
                customer_id: 'cust-3',
                location_id: 'loc-3',
              },
            }),
          ],
          linkRows: [
            link({
              id: 'link-cust-1',
              from_node_id: 'odp-node-1',
              to_node_id: 'customer-node-1',
            }),
            link({
              id: 'link-cust-2',
              from_node_id: 'odp-node-1',
              to_node_id: 'customer-node-2',
            }),
            link({
              id: 'link-cust-3',
              from_node_id: 'odp-node-1',
              to_node_id: 'customer-node-3',
            }),
          ],
        },
      ),
    ).toEqual({
      total: 8,
      used: 3,
      available: 5,
      state: 'partial',
    });
  });

  it('counts router uplink and downstream/customer links, but excludes splitter upstream, as used ODP ports', () => {
    const odp = asset({
      id: 'odp-1',
      asset_type: 'odp',
      metadata: {
        total_port_capacity: '8',
      },
    });

    expect(
      getNetworkAssetPortOccupancy(
        odp,
        [odp],
        {
          nodeRows: [
            node({
              id: 'odp-node-1',
              node_type: 'odp',
              metadata: {
                asset_source: 'network_asset',
                asset_type: 'odp',
                asset_id: 'odp-1',
              },
            }),
            node({
              id: 'customer-node-1',
              node_type: 'customer_premise',
              metadata: {
                asset_source: 'customer_location',
                customer_id: 'cust-1',
                location_id: 'loc-1',
              },
            }),
            node({
              id: 'downstream-odp-node-1',
              node_type: 'odp',
              metadata: {
                asset_source: 'network_asset',
                asset_type: 'odp',
                asset_id: 'odp-2',
              },
            }),
            node({
              id: 'splitter-node-1',
              node_type: 'splitter',
              metadata: {
                asset_source: 'network_asset',
                asset_type: 'splitter',
                asset_id: 'splitter-1',
              },
            }),
          ],
          linkRows: [
            link({
              from_node_id: 'odp-node-1',
              to_node_id: 'customer-node-1',
            }),
            link({
              from_node_id: 'odp-node-1',
              to_node_id: 'downstream-odp-node-1',
            }),
            link({
              from_node_id: 'splitter-node-1',
              to_node_id: 'odp-node-1',
            }),
            link({
              from_node_id: 'router-1',
              to_node_id: 'odp-node-1',
            }),
          ],
          routerRows: [
            router({
              id: 'router-1',
            }),
          ],
        },
      ),
    ).toEqual({
      total: 8,
      used: 3,
      available: 5,
      state: 'partial',
    });
  });

  it('counts maintenance or suspended customer-location links as used ports because the physical drop is still occupied', () => {
    const odp = asset({
      id: 'odp-1',
      metadata: {
        total_port_capacity: '8',
      },
    });

    expect(
      getNetworkAssetPortOccupancy(
        odp,
        [odp],
        {
          nodeRows: [
            node({
              id: 'odp-node-1',
              node_type: 'odp',
              metadata: {
                asset_source: 'network_asset',
                asset_type: 'odp',
                asset_id: 'odp-1',
              },
            }),
            node({
              id: 'customer-node-1',
              node_type: 'customer_premise',
              status: 'maintenance',
              metadata: {
                asset_source: 'customer_location',
                asset_type: 'customer_location',
                customer_id: 'cust-1',
                location_id: 'loc-1',
              },
            }),
          ],
          linkRows: [
            link({
              id: 'link-cust-1',
              from_node_id: 'odp-node-1',
              to_node_id: 'customer-node-1',
            }),
          ],
        },
      ),
    ).toEqual({
      total: 8,
      used: 1,
      available: 7,
      state: 'partial',
    });
  });

  it('counts active customer topology links by using a cached ODP node id when the asset node is absent from current node rows', () => {
    const odp = asset({
      id: 'odp-1',
      metadata: {
        total_port_capacity: '8',
      },
    });

    expect(
      getNetworkAssetPortOccupancy(
        odp,
        [odp],
        {
          assetNodeIdsByAssetId: new Map([['odp-1', 'odp-node-cached']]),
          nodeRows: [
            node({
              id: 'customer-node-1',
              node_type: 'customer_premise',
              status: 'active',
              metadata: {
                asset_source: 'customer_location',
                asset_type: 'customer_location',
                customer_id: 'cust-1',
                location_id: 'loc-1',
              },
            }),
          ],
          linkRows: [
            link({
              id: 'link-cust-1',
              from_node_id: 'odp-node-cached',
              to_node_id: 'customer-node-1',
            }),
          ],
        },
      ),
    ).toEqual({
      total: 8,
      used: 1,
      available: 7,
      state: 'partial',
    });
  });

  it('counts router uplink plus customer-side/downstream topology links as used ODP capacity', () => {
    const odp = asset({
      id: 'odp-1',
      metadata: {
        total_port_capacity: '8',
      },
    });

    expect(
      getNetworkAssetPortOccupancy(
        odp,
        [
          odp,
          asset({
            id: 'ont-1',
            asset_type: 'ont',
            parent_asset_id: 'odp-1',
            customer_id: 'cust-1',
            location_id: 'loc-1',
            status: 'installed',
          }),
        ],
        {
          nodeRows: [
            node({
              id: 'odp-node-1',
              node_type: 'odp',
              metadata: {
                asset_source: 'network_asset',
                asset_type: 'odp',
                asset_id: 'odp-1',
              },
            }),
            node({
              id: 'customer-node-1',
              node_type: 'customer_premise',
              status: 'active',
              metadata: {
                asset_source: 'customer_location',
                asset_type: 'customer_location',
                customer_id: 'cust-1',
                location_id: 'loc-1',
              },
            }),
            node({
              id: 'customer-node-2',
              node_type: 'customer_premise',
              status: 'active',
              metadata: {
                asset_source: 'customer_location',
                asset_type: 'customer_location',
                customer_id: 'cust-2',
                location_id: 'loc-2',
              },
            }),
            node({
              id: 'splitter-node-1',
              node_type: 'splitter',
              status: 'active',
              metadata: {
                asset_source: 'network_asset',
                asset_type: 'splitter',
                asset_id: 'splitter-1',
              },
            }),
          ],
          linkRows: [
            link({
              id: 'link-upstream-router',
              from_node_id: 'router-legacy-1',
              to_node_id: 'odp-node-1',
            }),
            link({
              id: 'link-upstream-splitter',
              from_node_id: 'splitter-node-1',
              to_node_id: 'odp-node-1',
            }),
            link({
              id: 'link-cust-1',
              from_node_id: 'odp-node-1',
              to_node_id: 'customer-node-1',
            }),
            link({
              id: 'link-cust-2',
              from_node_id: 'customer-node-2',
              to_node_id: 'odp-node-1',
            }),
          ],
          routerRows: [
            router({
              id: 'router-legacy-1',
              name: 'Core Router',
              identity: 'CORE-ROUTER',
            }),
          ],
        },
      ),
    ).toEqual({
      total: 8,
      used: 3,
      available: 5,
      state: 'partial',
    });
  });
});
