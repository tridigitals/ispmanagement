import { describe, expect, it } from 'vitest';

import type { NetworkAssetListItem } from '$lib/api/types';

import {
  assetSupportsCustomerDrop,
  assetSupportsUpstreamLink,
  buildTopologyAssetConnectionOperations,
  buildTopologyAssetConnectDraft,
  canTopologyAssetAcceptConnection,
  buildTopologyAssetParentOptions,
  findTopologyAssetNodeId,
  resolveTopologyAssetNodeId,
} from './networkMapAssetConnect';
import type { NMNode } from './networkMapUtils';

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
    created_at: '2026-05-14T00:00:00Z',
    updated_at: '2026-05-14T00:00:00Z',
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
    node_type: 'odp',
    status: 'active',
    lat: -7.2,
    lng: 110.3,
    metadata: {},
    ...overrides,
  };
}

describe('networkMapAssetConnect', () => {
  it('marks ODP-like assets as customer-drop capable', () => {
    expect(assetSupportsCustomerDrop('odp')).toBe(true);
    expect(assetSupportsCustomerDrop('fat')).toBe(true);
    expect(assetSupportsCustomerDrop('nap')).toBe(true);
    expect(assetSupportsCustomerDrop('odc')).toBe(false);
  });

  it('builds a draft from current asset relations', () => {
    expect(
      buildTopologyAssetConnectDraft(
        asset({
          asset_type: 'odp',
          parent_asset_id: 'odc-1',
          customer_id: 'cust-1',
          location_id: 'loc-1',
        }),
      ),
    ).toEqual({
      parentAssetId: 'odc-1',
      customerId: 'cust-1',
      locationId: 'loc-1',
    });
  });

  it('returns only eligible upstream asset choices for the current asset', () => {
    const options = buildTopologyAssetParentOptions({
      assetId: 'odp-1',
      assetType: 'odp',
      assets: [
        asset({ id: 'olt-1', asset_type: 'olt', name: 'OLT A' }),
        asset({ id: 'odc-1', asset_type: 'odc', name: 'ODC A' }),
        asset({ id: 'switch-1', asset_type: 'switch', name: 'Switch A' }),
        asset({ id: 'odp-1', asset_type: 'odp', name: 'ODP A' }),
        asset({ id: 'nap-1', asset_type: 'nap', name: 'NAP A' }),
      ],
    });

    expect(options.map((option) => option.value)).toEqual(['nap-1', 'odc-1', 'switch-1', 'olt-1']);
  });

  it('keeps the currently linked parent available even if it is outside the preferred hierarchy', () => {
    const options = buildTopologyAssetParentOptions({
      assetId: 'odc-1',
      assetType: 'odc',
      currentParentAssetId: 'nap-9',
      assets: [
        asset({ id: 'olt-1', asset_type: 'olt', name: 'OLT A' }),
        asset({ id: 'nap-9', asset_type: 'nap', name: 'NAP Legacy' }),
      ],
    });

    expect(options.map((option) => option.value)).toEqual(['nap-9', 'olt-1']);
  });

  it('marks mapped FTTH assets as upstream-link capable except terminal-like roots', () => {
    expect(assetSupportsUpstreamLink('odp')).toBe(true);
    expect(assetSupportsUpstreamLink('odc')).toBe(true);
    expect(assetSupportsUpstreamLink('splitter')).toBe(true);
    expect(assetSupportsUpstreamLink('odf')).toBe(true);
    expect(assetSupportsUpstreamLink('olt')).toBe(false);
  });

  it('rejects full ODP assets as connection targets while allowing non-capacity assets', () => {
    expect(
      canTopologyAssetAcceptConnection({
        assetType: 'odp',
        portCapacity: 8,
        portsAvailable: 0,
      }),
    ).toBe(false);
    expect(
      canTopologyAssetAcceptConnection({
        assetType: 'odp',
        portCapacity: 8,
        portsAvailable: 2,
      }),
    ).toBe(true);
    expect(
      canTopologyAssetAcceptConnection({
        assetType: 'odc',
        portCapacity: null,
        portsAvailable: null,
      }),
    ).toBe(true);
  });

  it('finds the synced topology node for an FTTH asset marker', () => {
    expect(
      findTopologyAssetNodeId(
        [
          node({
            id: 'ftth-node-1',
            metadata: {
              system_managed: true,
              asset_source: 'network_asset',
              asset_type: 'odp',
              asset_id: 'odp-1',
            },
          }),
        ],
        'odp-1',
      ),
    ).toBe('ftth-node-1');
  });

  it('falls back to nearby fetched rows when the current map page does not include the FTTH node', async () => {
    const syncCalls: string[] = [];
    const refreshCalls: string[] = [];
    const nearbyCalls: string[] = [];

    await expect(
      resolveTopologyAssetNodeId({
        assetId: 'odp-1',
        assetType: 'odp',
        latitude: -7.2,
        longitude: 110.3,
        nodeRows: [],
        syncNodes: async () => {
          syncCalls.push('sync');
        },
        refreshNodeRows: async () => {
          refreshCalls.push('refresh');
          return [];
        },
        fetchNearbyNodeRows: async () => {
          nearbyCalls.push('nearby');
          return [
            node({
              id: 'ftth-node-nearby',
              metadata: {
                system_managed: true,
                asset_source: 'network_asset',
                asset_type: 'odp',
                asset_id: 'odp-1',
              },
            }),
          ];
        },
      }),
    ).resolves.toBe('ftth-node-nearby');

    expect(syncCalls).toEqual(['sync']);
    expect(refreshCalls).toEqual(['refresh']);
    expect(nearbyCalls).toEqual(['nearby']);
  });

  it('reuses a previously resolved FTTH node id before forcing another sync cycle', async () => {
    const syncCalls: string[] = [];

    await expect(
      resolveTopologyAssetNodeId({
        assetId: 'odp-1',
        assetType: 'odp',
        latitude: -7.2,
        longitude: 110.3,
        nodeRows: [],
        cachedNodeId: 'ftth-node-cached',
        syncNodes: async () => {
          syncCalls.push('sync');
        },
        refreshNodeRows: async () => [],
        fetchNearbyNodeRows: async () => [],
      }),
    ).resolves.toBe('ftth-node-cached');

    expect(syncCalls).toEqual([]);
  });

  it('does not collapse ODP customer-drop links back into a single customer/location relation on the source asset', () => {
    expect(
      buildTopologyAssetConnectionOperations({
        sourceAsset: asset({
          id: 'odp-1',
          asset_type: 'odp',
        }),
        targetNode: node({
          id: 'cust-node-1',
          node_type: 'customer_premise',
          metadata: {
            asset_source: 'customer_location',
            asset_type: 'customer_location',
            asset_id: 'loc-1',
            customer_id: 'cust-1',
            location_id: 'loc-1',
          },
        }),
      }),
    ).toEqual([]);
  });

  it('assigns the downstream FTTH asset to the upstream parent regardless of draw direction', () => {
    expect(
      buildTopologyAssetConnectionOperations({
        sourceAsset: asset({
          id: 'odc-1',
          asset_type: 'odc',
        }),
        targetNode: node({
          id: 'odp-node-1',
          node_type: 'odp',
          metadata: {
            asset_source: 'network_asset',
            asset_type: 'odp',
            asset_id: 'odp-1',
          },
        }),
      }),
    ).toEqual([
      {
        assetId: 'odp-1',
        parentAssetId: 'odc-1',
      },
    ]);
  });

  it('assigns ODP upstream to splitter assets when connected on the topology map', () => {
    expect(
      buildTopologyAssetConnectionOperations({
        sourceAsset: asset({
          id: 'odp-1',
          asset_type: 'odp',
        }),
        targetNode: node({
          id: 'splitter-node-1',
          node_type: 'splitter',
          metadata: {
            asset_source: 'network_asset',
            asset_type: 'splitter',
            asset_id: 'splitter-1',
          },
        }),
      }),
    ).toEqual([
      {
        assetId: 'odp-1',
        parentAssetId: 'splitter-1',
      },
    ]);
  });
});
