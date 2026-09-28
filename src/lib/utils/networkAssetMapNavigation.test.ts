import { describe, expect, it } from 'vitest';

import {
  buildNetworkAssetMapUrl,
  parseNetworkAssetMapTarget,
} from './networkAssetMapNavigation';

describe('network asset map navigation', () => {
  it('builds a topology map URL carrying the asset focus target', () => {
    expect(
      buildNetworkAssetMapUrl({
        tenantPrefix: '',
        assetId: 'asset-1',
        latitude: -6.5560834,
        longitude: 106.9217893,
      }),
    ).toBe(
      '/admin/network/map?asset_id=asset-1&asset_lat=-6.5560834&asset_lng=106.9217893',
    );
  });

  it('parses a valid asset focus target from search params', () => {
    const params = new URLSearchParams({
      asset_id: 'asset-1',
      asset_lat: '-6.5560834',
      asset_lng: '106.9217893',
    });

    expect(parseNetworkAssetMapTarget(params)).toEqual({
      assetId: 'asset-1',
      latitude: -6.5560834,
      longitude: 106.9217893,
    });
  });
});
