import { describe, expect, it, vi } from 'vitest';

import { registerMapSourcesAndLayers, replaceTopologyAssetSourceData } from './networkMapLayers';

describe('registerMapSourcesAndLayers', () => {
  it('enables clustering for customer, router, and topology asset point sources', () => {
    const addedSources = new Map<string, any>();
    const map = {
      addSource: vi.fn((id: string, source: any) => {
        addedSources.set(id, source);
      }),
      addLayer: vi.fn(),
      getSource: vi.fn(() => undefined),
      getLayer: vi.fn(() => undefined),
      getStyle: vi.fn(() => ({ glyphs: 'https://example.test/fonts/{fontstack}/{range}.pbf' })),
    } as unknown as import('maplibre-gl').Map;

    registerMapSourcesAndLayers(map);

    expect(addedSources.get('nm-customers')).toMatchObject({
      cluster: true,
      clusterMaxZoom: 14,
      clusterRadius: 54,
    });
    expect(addedSources.get('nm-routers')).toMatchObject({
      cluster: true,
      clusterMaxZoom: 14,
      clusterRadius: 52,
    });
    expect(addedSources.get('nm-topology-assets')).toMatchObject({
      cluster: true,
      clusterMaxZoom: 14,
      clusterRadius: 56,
    });
  });

  it('registers topology asset layers after customer layers so asset markers stay visible', () => {
    const layerIds: string[] = [];
    let hasTopologySource = false;
    const map = {
      addSource: vi.fn((id: string) => {
        if (id === 'nm-topology-assets') hasTopologySource = true;
      }),
      addLayer: vi.fn((layer: { id: string }) => {
        layerIds.push(layer.id);
      }),
      getSource: vi.fn((id: string) =>
        id === 'nm-topology-assets' && hasTopologySource ? ({} as any) : undefined,
      ),
      getLayer: vi.fn(() => undefined),
      getStyle: vi.fn(() => ({ glyphs: 'https://example.test/fonts/{fontstack}/{range}.pbf' })),
    } as unknown as import('maplibre-gl').Map;

    registerMapSourcesAndLayers(map);

    expect(layerIds.indexOf('nm-topology-assets-circle')).toBeGreaterThan(
      layerIds.indexOf('nm-customers-point'),
    );
    expect(layerIds.indexOf('nm-topology-assets-halo')).toBeGreaterThan(
      layerIds.indexOf('nm-customers-point'),
    );
    expect(layerIds.indexOf('nm-topology-assets-circle')).toBeGreaterThan(
      layerIds.indexOf('nm-topology-assets-halo'),
    );
    expect(layerIds.indexOf('nm-topology-assets-icon')).toBeGreaterThan(
      layerIds.indexOf('nm-topology-assets-circle'),
    );
    expect(layerIds).toContain('nm-routers-cluster-circle');
    expect(layerIds).toContain('nm-topology-assets-cluster-circle');
  });

  it('recreates topology asset source before re-adding layers', () => {
    const removedLayers: string[] = [];
    const addedLayerIds: string[] = [];
    let hasSource = true;
    const map = {
      addSource: vi.fn(),
      removeSource: vi.fn(() => {
        hasSource = false;
      }),
      getSource: vi.fn(() => (hasSource ? ({} as any) : undefined)),
      addLayer: vi.fn((layer: { id: string }) => {
        addedLayerIds.push(layer.id);
      }),
      getLayer: vi.fn((id: string) =>
        [
          'nm-topology-assets-cluster-count',
          'nm-topology-assets-cluster-circle',
          'nm-topology-assets-halo',
          'nm-topology-assets-circle',
          'nm-topology-assets-icon',
          'nm-topology-assets-label',
        ].includes(id)
          ? ({ id } as any)
          : undefined,
      ),
      removeLayer: vi.fn((id: string) => {
        removedLayers.push(id);
      }),
      getStyle: vi.fn(() => ({ glyphs: 'https://example.test/fonts/{fontstack}/{range}.pbf' })),
    } as unknown as import('maplibre-gl').Map;

    replaceTopologyAssetSourceData(map, {
      type: 'FeatureCollection',
      features: [],
    });

    expect(removedLayers).toEqual([
      'nm-topology-assets-label',
      'nm-topology-assets-cluster-count',
      'nm-topology-assets-cluster-circle',
      'nm-topology-assets-icon',
      'nm-topology-assets-circle',
      'nm-topology-assets-halo',
    ]);
    expect(map.removeSource).toHaveBeenCalled();
    expect(map.addSource).toHaveBeenCalled();
    expect(map.addSource).toHaveBeenCalledWith(
      'nm-topology-assets',
      expect.objectContaining({
        cluster: true,
        clusterMaxZoom: 14,
        clusterRadius: 56,
      }),
    );
    expect(addedLayerIds).toContain('nm-topology-assets-cluster-circle');
    expect(addedLayerIds).toContain('nm-topology-assets-halo');
    expect(addedLayerIds).toContain('nm-topology-assets-circle');
    expect(addedLayerIds).toContain('nm-topology-assets-icon');
  });
});
