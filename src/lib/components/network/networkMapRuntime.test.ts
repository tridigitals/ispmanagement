import { describe, expect, it, vi } from 'vitest';

import { fitMapToMarkers } from './networkMapRuntime';

describe('fitMapToMarkers', () => {
  it('includes topology asset coordinates in the first auto-fit bounds', () => {
    const fitBounds = vi.fn();
    const map = { fitBounds } as unknown as import('maplibre-gl').Map;

    class FakeLngLatBounds {
      points: Array<[number, number]>;

      constructor(sw: [number, number], ne: [number, number]) {
        this.points = [sw, ne];
      }

      extend(point: [number, number]) {
        this.points.push(point);
        return this;
      }
    }

    const didFit = fitMapToMarkers({
      map,
      maplibre: { LngLatBounds: FakeLngLatBounds } as unknown as typeof import('maplibre-gl'),
      didInitialFitToMarkers: false,
      nodes: [],
      routers: [],
      topologyAssets: [
        {
          id: 'asset-1',
          name: 'ODP Test',
          assetType: 'odp',
          assetTypeLabel: 'ODP',
          status: 'available',
          code: null,
          serialNumber: null,
          customerName: null,
          locationLabel: null,
          latitude: -6.2088,
          longitude: 106.8456,
          customerId: null,
          locationId: null,
          parentAssetId: null,
          markerLabel: 'ODP',
          markerColor: '#0f766e',
          legendLabel: 'ODP',
          portCapacity: 8,
          portsUsed: 1,
          portsAvailable: 7,
          canAcceptConnections: true,
          hasUpstreamRelation: false,
          hasCustomerRelation: true,
        },
      ],
      installationTargetCoord: null,
    });

    expect(didFit).toBe(true);
    expect(fitBounds).toHaveBeenCalledTimes(1);
    const [bounds] = fitBounds.mock.calls[0];
    expect(bounds.points).toContainEqual([106.8456, -6.2088]);
  });
});
