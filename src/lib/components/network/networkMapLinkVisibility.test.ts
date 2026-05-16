import { describe, expect, it } from 'vitest';

import { buildLinkLayerVisibilityState } from './networkMapLinkVisibility';

describe('buildLinkLayerVisibilityState', () => {
  it('keeps link layers visible when connect focus is not active', () => {
    expect(
      buildLinkLayerVisibilityState({
        linksVisible: true,
        topologyAssetsVisible: true,
        linkPickMode: false,
        activeAssetConnectSourceId: null,
      }),
    ).toEqual({
      mainLinksVisible: true,
      topologyAssetLinksVisible: true,
    });
  });

  it('hides all map link layers while drawing from a topology asset connect action', () => {
    expect(
      buildLinkLayerVisibilityState({
        linksVisible: true,
        topologyAssetsVisible: true,
        linkPickMode: true,
        activeAssetConnectSourceId: 'odp-1',
      }),
    ).toEqual({
      mainLinksVisible: false,
      topologyAssetLinksVisible: false,
    });
  });

  it('does not hide map links during generic link drawing when no topology asset connect source is active', () => {
    expect(
      buildLinkLayerVisibilityState({
        linksVisible: true,
        topologyAssetsVisible: true,
        linkPickMode: true,
        activeAssetConnectSourceId: null,
      }),
    ).toEqual({
      mainLinksVisible: true,
      topologyAssetLinksVisible: true,
    });
  });
});
