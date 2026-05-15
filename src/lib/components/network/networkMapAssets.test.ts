import { describe, expect, it } from 'vitest';

import type { NetworkAssetListItem } from '$lib/api/types';

import type { NMLink, NMNode } from './networkMapUtils';
import {
  buildTopologyAssetAutoLinkFeatureCollection,
  buildTopologyAssetRows,
} from './networkMapAssets';

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
    latitude: -7.2,
    longitude: 110.3,
    notes: null,
    metadata: {},
    created_at: '2026-05-13T00:00:00Z',
    updated_at: '2026-05-13T00:00:00Z',
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
    name: 'Customer Premise',
    node_type: 'customer_premise',
    status: 'active',
    lat: -7.21,
    lng: 110.31,
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

describe('networkMapAssets', () => {
  it('enriches ODP rows with occupancy data', () => {
    const rows = buildTopologyAssetRows([
      asset({
        id: 'odp-1',
        asset_type: 'odp',
        metadata: { total_port_capacity: '8' },
      }),
      asset({
        id: 'ont-1',
        asset_type: 'ont',
        parent_asset_id: 'odp-1',
        customer_id: 'cust-1',
        location_id: 'loc-1',
        status: 'installed',
      }),
    ]);

    expect(rows.find((row) => row.id === 'odp-1')).toMatchObject({
      portCapacity: 8,
      portsUsed: 1,
      portsAvailable: 7,
      hasUpstreamRelation: false,
      hasCustomerRelation: true,
    });
  });

  it('marks standalone ODP rows as not yet linked upstream or to customer drops', () => {
    const rows = buildTopologyAssetRows([
      asset({
        id: 'odp-1',
        asset_type: 'odp',
        metadata: { total_port_capacity: '8' },
      }),
    ]);

    expect(rows.find((row) => row.id === 'odp-1')).toMatchObject({
      hasUpstreamRelation: false,
      hasCustomerRelation: false,
    });
  });

  it('builds auto links between parent assets and customer endpoints', () => {
    const assets = [
      asset({
        id: 'odc-1',
        asset_type: 'odc',
        name: 'ODC-1',
        latitude: -7.2,
        longitude: 110.3,
      }),
      asset({
        id: 'odp-1',
        asset_type: 'odp',
        name: 'ODP-1',
        parent_asset_id: 'odc-1',
        latitude: -7.21,
        longitude: 110.31,
        metadata: { total_port_capacity: '8' },
      }),
      asset({
        id: 'ont-1',
        asset_type: 'ont',
        parent_asset_id: 'odp-1',
        customer_id: 'cust-1',
        location_id: 'loc-1',
        status: 'installed',
      }),
    ];

    const rows = buildTopologyAssetRows(assets);
    const fc = buildTopologyAssetAutoLinkFeatureCollection({
      assets,
      topologyRows: rows,
      customerNodes: [
        node({
          id: 'cust-node-1',
          lat: -7.215,
          lng: 110.315,
          metadata: {
            customer_id: 'cust-1',
            location_id: 'loc-1',
          },
        }),
      ],
      nodeRows: [],
      linkRows: [],
    });

    expect(fc.features).toHaveLength(2);
    expect(fc.features.map((feature) => feature.properties?.link_kind)).toEqual([
      'asset_parent',
      'customer_drop',
    ]);
  });

  it('builds a direct customer drop for ODP rows linked straight to a customer location', () => {
    const assets = [
      asset({
        id: 'odp-1',
        asset_type: 'odp',
        name: 'ODP-1',
        customer_id: 'cust-1',
        location_id: 'loc-1',
        latitude: -7.21,
        longitude: 110.31,
        metadata: { total_port_capacity: '8' },
      }),
    ];

    const rows = buildTopologyAssetRows(assets);
    const fc = buildTopologyAssetAutoLinkFeatureCollection({
      assets,
      topologyRows: rows,
      customerNodes: [
        node({
          id: 'cust-node-1',
          lat: -7.215,
          lng: 110.315,
          metadata: {
            customer_id: 'cust-1',
            location_id: 'loc-1',
          },
        }),
      ],
      nodeRows: [],
      linkRows: [],
    });

    expect(fc.features).toHaveLength(1);
    expect(fc.features[0]?.properties?.link_kind).toBe('customer_drop');
  });

  it('skips helper auto-links when a real saved topology link already exists', () => {
    const assets = [
      asset({
        id: 'odc-1',
        asset_type: 'odc',
        name: 'ODC-1',
        latitude: -7.2,
        longitude: 110.3,
      }),
      asset({
        id: 'odp-1',
        asset_type: 'odp',
        name: 'ODP-1',
        parent_asset_id: 'odc-1',
        customer_id: 'cust-1',
        location_id: 'loc-1',
        latitude: -7.21,
        longitude: 110.31,
        metadata: { total_port_capacity: '8' },
      }),
    ];

    const rows = buildTopologyAssetRows(assets);
    const fc = buildTopologyAssetAutoLinkFeatureCollection({
      assets,
      topologyRows: rows,
      customerNodes: [
        node({
          id: 'cust-node-1',
          lat: -7.215,
          lng: 110.315,
          metadata: {
            customer_id: 'cust-1',
            location_id: 'loc-1',
          },
        }),
      ],
      nodeRows: [
        node({
          id: 'odc-node-1',
          node_type: 'odc',
          metadata: {
            asset_source: 'network_asset',
            asset_type: 'odc',
            asset_id: 'odc-1',
          },
        }),
        node({
          id: 'odp-node-1',
          node_type: 'odp',
          metadata: {
            asset_source: 'network_asset',
            asset_type: 'odp',
            asset_id: 'odp-1',
          },
        }),
      ],
      linkRows: [
        link({
          id: 'link-parent-1',
          from_node_id: 'odc-node-1',
          to_node_id: 'odp-node-1',
        }),
        link({
          id: 'link-drop-1',
          from_node_id: 'odp-node-1',
          to_node_id: 'cust-node-1',
        }),
      ],
    });

    expect(fc.features).toHaveLength(0);
  });
});
