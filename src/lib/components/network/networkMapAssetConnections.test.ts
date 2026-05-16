import { describe, expect, it } from 'vitest';

import { buildTopologyAssetConnectionItems } from './networkMapAssetConnections';
import type { NMLink, NMNode } from './networkMapUtils';

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

describe('networkMapAssetConnections', () => {
  it('builds topology-based upstream and connected summaries for an ODP asset', () => {
    expect(
      buildTopologyAssetConnectionItems({
        asset: {
          id: 'odp-1',
          asset_type: 'odp',
          parent_asset_name: 'ODC-1',
          customer_name: null,
          location_label: null,
        } as any,
        topologyAssets: [
          { id: 'odc-1', name: 'ODC-1' },
          { id: 'odp-1', name: 'ODP-1' },
        ] as any,
        assetNodeIdsByAssetId: new Map([['odp-1', 'odp-node-1']]),
        nodeRows: [
          node({
            id: 'odc-node-1',
            node_type: 'odc',
            metadata: {
              asset_source: 'network_asset',
              asset_id: 'odc-1',
            },
          }),
          node({
            id: 'odp-node-1',
            node_type: 'odp',
            metadata: {
              asset_source: 'network_asset',
              asset_id: 'odp-1',
            },
          }),
          node({
            id: 'cust-node-1',
            node_type: 'customer_premise',
            name: 'Andi Home',
            metadata: {
              asset_source: 'customer_location',
              customer_name: 'Andi',
              location_label: 'Rumah Andi',
            },
          }),
          node({
            id: 'cust-node-2',
            node_type: 'customer_premise',
            name: 'Budi Home',
            metadata: {
              asset_source: 'customer_location',
              customer_name: 'Budi',
            },
          }),
        ],
        linkRows: [
          link({
            id: 'upstream-link',
            from_node_id: 'odc-node-1',
            to_node_id: 'odp-node-1',
          }),
          link({
            id: 'customer-link-1',
            from_node_id: 'odp-node-1',
            to_node_id: 'cust-node-1',
          }),
          link({
            id: 'customer-link-2',
            from_node_id: 'odp-node-1',
            to_node_id: 'cust-node-2',
          }),
        ],
      }),
    ).toEqual([
      { label: 'Upstream', value: 'ODC-1' },
      { label: 'Ports Used', value: '2 endpoint linked' },
      { label: 'Connected', value: 'Andi, Budi' },
    ]);
  });
});
