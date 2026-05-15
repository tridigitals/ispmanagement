import { describe, expect, it } from 'vitest';

import type { NetworkAssetListItem } from '$lib/api/types';

import {
  assetSupportsCustomerDrop,
  assetSupportsUpstreamLink,
  buildTopologyAssetConnectionOperations,
  buildTopologyAssetConnectDraft,
  buildTopologyAssetParentOptions,
  findTopologyAssetNodeId,
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

    expect(options.map((option) => option.value)).toEqual(['odc-1', 'nap-1', 'switch-1', 'olt-1']);
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
    expect(assetSupportsUpstreamLink('olt')).toBe(false);
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

  it('maps FTTH-to-customer links back into asset customer and location relations', () => {
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
    ).toEqual([
      {
        assetId: 'odp-1',
        customerId: 'cust-1',
        locationId: 'loc-1',
      },
    ]);
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
});
