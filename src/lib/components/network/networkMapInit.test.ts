import { describe, expect, it, vi } from 'vitest';

import { registerPrimaryLayerClicks } from './networkMapInit';

describe('registerPrimaryLayerClicks', () => {
  it('registers cluster click handlers for customer, router, and topology assets', () => {
    const on = vi.fn();
    const map = { on } as unknown as import('maplibre-gl').Map;
    const onNodeClick = vi.fn();
    const onRouterClick = vi.fn();
    const onTopologyAssetClick = vi.fn();
    const onLinkClick = vi.fn();
    const onCustomerClusterClick = vi.fn();
    const onRouterClusterClick = vi.fn();
    const onTopologyAssetClusterClick = vi.fn();

    registerPrimaryLayerClicks({
      map,
      onNodeClick,
      onRouterClick,
      onTopologyAssetClick,
      onLinkClick,
      onCustomerClusterClick,
      onRouterClusterClick,
      onTopologyAssetClusterClick,
    });

    expect(on).toHaveBeenCalledWith('click', 'nm-customers-cluster-circle', onCustomerClusterClick);
    expect(on).toHaveBeenCalledWith('click', 'nm-customers-cluster-count', onCustomerClusterClick);
    expect(on).toHaveBeenCalledWith('click', 'nm-routers-cluster-circle', onRouterClusterClick);
    expect(on).toHaveBeenCalledWith('click', 'nm-routers-cluster-count', onRouterClusterClick);
    expect(on).toHaveBeenCalledWith(
      'click',
      'nm-topology-assets-cluster-circle',
      onTopologyAssetClusterClick,
    );
    expect(on).toHaveBeenCalledWith(
      'click',
      'nm-topology-assets-cluster-count',
      onTopologyAssetClusterClick,
    );
  });
});
