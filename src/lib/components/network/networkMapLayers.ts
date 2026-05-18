import type { FeatureCollection } from 'geojson';

export const SOURCE_NODES = 'nm-nodes';
export const SOURCE_CUSTOMERS = 'nm-customers';
export const SOURCE_LINKS = 'nm-links';
export const SOURCE_ZONES = 'nm-zones';
export const SOURCE_ROUTERS = 'nm-routers';
export const SOURCE_TOPOLOGY_ASSETS = 'nm-topology-assets';
export const SOURCE_TOPOLOGY_ASSET_LINKS = 'nm-topology-asset-links';
export const SOURCE_LINK_DRAFT = 'nm-link-draft';
export const SOURCE_LINK_DRAFT_POINTS = 'nm-link-draft-points';
export const SOURCE_SELECTION_POINTS = 'nm-selection-points';
export const SOURCE_SELECTION_LINES = 'nm-selection-lines';
export const SOURCE_SELECTION_ZONES = 'nm-selection-zones';

export function emptyFeatureCollection(): FeatureCollection {
  return { type: 'FeatureCollection', features: [] };
}

export function buildBaseMapStyle({
  hasHiResSatellite,
  mapTilerKey,
  standardMaxZoom,
  satelliteMaxZoom,
}: {
  hasHiResSatellite: boolean;
  mapTilerKey: string | undefined;
  standardMaxZoom: number;
  satelliteMaxZoom: number;
}) {
  return {
    version: 8 as const,
    glyphs: 'https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf',
    sources: {
      osm: {
        type: 'raster' as const,
        tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
        tileSize: 256,
        maxzoom: standardMaxZoom,
        attribution: '© OpenStreetMap contributors',
      },
      satellite: {
        type: 'raster' as const,
        tiles: hasHiResSatellite
          ? [`https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=${mapTilerKey}`]
          : [
              'https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
            ],
        tileSize: 256,
        maxzoom: satelliteMaxZoom,
        attribution: hasHiResSatellite ? '© MapTiler © OpenStreetMap contributors' : '© Esri',
      },
    },
    layers: [
      { id: 'base-standard', type: 'raster' as const, source: 'osm' },
      {
        id: 'base-satellite',
        type: 'raster' as const,
        source: 'satellite',
        layout: { visibility: 'none' as const },
      },
    ],
  };
}

function addTopologyAssetLayers(map: import('maplibre-gl').Map) {
  map.addLayer({
    id: 'nm-topology-assets-halo',
    type: 'circle',
    source: SOURCE_TOPOLOGY_ASSETS,
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 9.5, 11, 12, 14, 14.5],
      'circle-color': ['coalesce', ['get', 'marker_color'], '#64748b'],
      'circle-opacity': 0.16,
      'circle-blur': 0.08,
      'circle-stroke-width': 0,
    },
  });

  map.addLayer({
    id: 'nm-topology-assets-circle',
    type: 'circle',
    source: SOURCE_TOPOLOGY_ASSETS,
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 7.4, 11, 9.2, 14, 11.2],
      'circle-color': ['coalesce', ['get', 'marker_color'], '#64748b'],
      'circle-opacity': 0.38,
      'circle-stroke-width': 2,
      'circle-stroke-color': '#e2e8f0',
    },
  });

  map.addLayer({
    id: 'nm-topology-assets-icon',
    type: 'symbol',
    source: SOURCE_TOPOLOGY_ASSETS,
    layout: {
      'icon-image': [
        'match',
        ['get', 'asset_type'],
        'olt',
        'nm-node-glyph-olt',
        'odf',
        'nm-node-glyph-odf',
        'odc',
        'nm-node-glyph-odc',
        'odp',
        'nm-node-glyph-odp',
        'fat',
        'nm-node-glyph-odp',
        'nap',
        'nm-node-glyph-odp',
        'splitter',
        'nm-node-glyph-splitter',
        'switch',
        'nm-node-glyph-switch',
        'nm-node-glyph-router',
      ],
      'icon-size': ['interpolate', ['linear'], ['zoom'], 8, 1.14, 11, 1.3, 14, 1.46],
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
    },
  });
}

export function ensureTopologyAssetSourceAndLayers(map: import('maplibre-gl').Map) {
  if (!map.getSource(SOURCE_TOPOLOGY_ASSETS)) {
    map.addSource(SOURCE_TOPOLOGY_ASSETS, { type: 'geojson', data: emptyFeatureCollection() });
  }
  if (!map.getLayer('nm-topology-assets-circle')) {
    addTopologyAssetLayers(map);
  }
}

export function replaceTopologyAssetSourceData(
  map: import('maplibre-gl').Map,
  data: FeatureCollection = emptyFeatureCollection(),
) {
  for (const layerId of [
    'nm-topology-assets-label',
    'nm-topology-assets-icon',
    'nm-topology-assets-circle',
    'nm-topology-assets-halo',
  ]) {
    if (map.getLayer(layerId)) map.removeLayer(layerId);
  }

  if (map.getSource(SOURCE_TOPOLOGY_ASSETS)) {
    map.removeSource(SOURCE_TOPOLOGY_ASSETS);
  }

  map.addSource(SOURCE_TOPOLOGY_ASSETS, {
    type: 'geojson',
    data: JSON.parse(JSON.stringify(data)),
  });
  addTopologyAssetLayers(map);
}

export function registerMapSourcesAndLayers(map: import('maplibre-gl').Map) {
  map.addSource(SOURCE_ZONES, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_LINKS, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_NODES, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_ROUTERS, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_TOPOLOGY_ASSET_LINKS, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_CUSTOMERS, {
    type: 'geojson',
    data: emptyFeatureCollection(),
    cluster: true,
    clusterMaxZoom: 14,
    clusterRadius: 54,
  });
  map.addSource(SOURCE_LINK_DRAFT, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_LINK_DRAFT_POINTS, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_SELECTION_POINTS, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_SELECTION_LINES, { type: 'geojson', data: emptyFeatureCollection() });
  map.addSource(SOURCE_SELECTION_ZONES, { type: 'geojson', data: emptyFeatureCollection() });

  map.addLayer({
    id: 'nm-zones-fill',
    type: 'fill',
    source: SOURCE_ZONES,
    paint: {
      'fill-color': '#1fb6ff',
      'fill-opacity': 0.12,
    },
  });

  map.addLayer({
    id: 'nm-zones-outline',
    type: 'line',
    source: SOURCE_ZONES,
    paint: {
      'line-color': '#1fb6ff',
      'line-width': 2,
      'line-opacity': 0.9,
    },
  });

  map.addLayer({
    id: 'nm-links-line',
    type: 'line',
    source: SOURCE_LINKS,
    filter: [
      'all',
      ['!=', ['get', 'status'], 'maintenance'],
      ['!=', ['get', 'status'], 'planning'],
    ],
    paint: {
      'line-color': [
        'match',
        ['get', 'health_tone'],
        'bad',
        '#ef4444',
        'warn',
        '#f59e0b',
        '#3f8cff',
      ],
      'line-width': 2.5,
      'line-opacity': 0.95,
    },
  });

  map.addLayer({
    id: 'nm-links-line-dashed',
    type: 'line',
    source: SOURCE_LINKS,
    filter: ['in', ['get', 'status'], ['literal', ['maintenance', 'planning']]],
    paint: {
      'line-color': [
        'match',
        ['get', 'health_tone'],
        'bad',
        '#ef4444',
        'warn',
        '#f59e0b',
        '#3f8cff',
      ],
      'line-width': 2.5,
      'line-opacity': 0.95,
      'line-dasharray': [1.4, 1.2],
    },
  });

  map.addLayer({
    id: 'nm-nodes-circle',
    type: 'circle',
    source: SOURCE_NODES,
    filter: [
      'all',
      ['!=', ['get', 'node_type'], 'customer_endpoint'],
      ['!=', ['get', 'node_type'], 'customer_premise'],
    ],
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 6, 11, 8, 14, 10.5],
      'circle-color': [
        'match',
        ['get', 'status'],
        'active',
        '#16a34a',
        'maintenance',
        '#f59e0b',
        '#64748b',
      ],
      'circle-stroke-width': 1.6,
      'circle-stroke-color': '#e2e8f0',
    },
  });

  map.addLayer({
    id: 'nm-nodes-icons',
    type: 'symbol',
    source: SOURCE_NODES,
    filter: [
      'all',
      ['!=', ['get', 'node_type'], 'customer_endpoint'],
      ['!=', ['get', 'node_type'], 'customer_premise'],
    ],
    layout: {
      'icon-image': [
        'match',
        ['get', 'node_type'],
        'core',
        'nm-node-icon-core',
        'pop',
        'nm-node-icon-pop',
        'olt',
        'nm-node-icon-olt',
        'router',
        'nm-node-icon-router',
        'switch',
        'nm-node-icon-switch',
        'tower',
        'nm-node-icon-tower',
        'ap',
        'nm-node-icon-ap',
        'odc',
        'nm-node-icon-odc',
        'odp',
        'nm-node-icon-odp',
        'splitter',
        'nm-node-icon-splitter',
        'junction',
        'nm-node-icon-junction',
        'customer_premise',
        'nm-node-icon-customer',
        'customer_endpoint',
        'nm-node-icon-customer',
        'nm-node-icon-router',
      ],
      'icon-size': ['interpolate', ['linear'], ['zoom'], 8, 0.58, 11, 0.72, 14, 0.88],
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
    },
  });

  map.addLayer({
    id: 'nm-routers-circle',
    type: 'circle',
    source: SOURCE_ROUTERS,
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 7, 11, 9, 14, 11.5],
      'circle-color': ['case', ['==', ['get', 'is_online'], true], '#16a34a', '#ef4444'],
      'circle-stroke-width': 2,
      'circle-stroke-color': '#e2e8f0',
    },
  });

  map.addLayer({
    id: 'nm-routers-icon',
    type: 'symbol',
    source: SOURCE_ROUTERS,
    layout: {
      'icon-image': 'nm-node-icon-router',
      'icon-size': ['interpolate', ['linear'], ['zoom'], 8, 0.72, 11, 0.86, 14, 1],
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
    },
  });

  map.addLayer({
    id: 'nm-customers-cluster-circle',
    type: 'circle',
    source: SOURCE_CUSTOMERS,
    filter: ['has', 'point_count'],
    paint: {
      'circle-color': ['step', ['get', 'point_count'], '#06b6d4', 20, '#3b82f6', 60, '#6366f1'],
      'circle-radius': ['step', ['get', 'point_count'], 16, 20, 20, 60, 24],
      'circle-stroke-width': 1.6,
      'circle-stroke-color': '#e2e8f0',
    },
  });

  if (map.getStyle()?.glyphs) {
    map.addLayer({
      id: 'nm-customers-cluster-count',
      type: 'symbol',
      source: SOURCE_CUSTOMERS,
      filter: ['has', 'point_count'],
      layout: {
        'text-field': ['to-string', ['get', 'point_count_abbreviated']],
        'text-size': 11,
        'text-allow-overlap': true,
      },
      paint: {
        'text-color': '#f8fafc',
      },
    });
  }

  map.addLayer({
    id: 'nm-customers-point',
    type: 'symbol',
    source: SOURCE_CUSTOMERS,
    filter: ['!', ['has', 'point_count']],
    layout: {
      'icon-image': [
        'match',
        ['get', 'pppoe_visual_state'],
        'connected',
        'nm-node-icon-customer-connected',
        'disconnected',
        'nm-node-icon-customer-disconnected',
        'neutral',
        'nm-node-icon-customer-neutral',
        'nm-node-icon-customer-neutral',
      ],
      'icon-size': ['interpolate', ['linear'], ['zoom'], 8, 0.64, 11, 0.8, 14, 0.94],
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
    },
  });

  map.addLayer({
    id: 'nm-topology-asset-links-parent',
    type: 'line',
    source: SOURCE_TOPOLOGY_ASSET_LINKS,
    filter: ['==', ['get', 'link_kind'], 'asset_parent'],
    paint: {
      'line-color': '#60a5fa',
      'line-width': ['interpolate', ['linear'], ['zoom'], 8, 1.8, 11, 2.2, 14, 2.8],
      'line-opacity': 0.88,
      'line-dasharray': [2.1, 1.2],
    },
  });

  map.addLayer({
    id: 'nm-topology-asset-links-customer',
    type: 'line',
    source: SOURCE_TOPOLOGY_ASSET_LINKS,
    filter: ['==', ['get', 'link_kind'], 'customer_drop'],
    paint: {
      'line-color': '#14b8a6',
      'line-width': ['interpolate', ['linear'], ['zoom'], 8, 1.8, 11, 2.2, 14, 2.8],
      'line-opacity': 0.88,
      'line-dasharray': [1.1, 1.1],
    },
  });

  ensureTopologyAssetSourceAndLayers(map);

  map.addLayer({
    id: 'nm-link-draft-line',
    type: 'line',
    source: SOURCE_LINK_DRAFT,
    paint: {
      'line-color': '#38bdf8',
      'line-width': 3,
      'line-opacity': 0.9,
      'line-dasharray': [1.5, 1.2],
    },
  });

  map.addLayer({
    id: 'nm-link-draft-points',
    type: 'circle',
    source: SOURCE_LINK_DRAFT_POINTS,
    paint: {
      'circle-radius': 4.5,
      'circle-color': '#38bdf8',
      'circle-stroke-color': '#0b1020',
      'circle-stroke-width': 1.2,
    },
  });

  map.addLayer({
    id: 'nm-selection-zones-outline',
    type: 'line',
    source: SOURCE_SELECTION_ZONES,
    paint: {
      'line-color': '#f8fafc',
      'line-width': 4,
      'line-opacity': 0.95,
    },
  });

  map.addLayer({
    id: 'nm-selection-lines',
    type: 'line',
    source: SOURCE_SELECTION_LINES,
    paint: {
      'line-color': '#f8fafc',
      'line-width': 5,
      'line-opacity': 0.95,
    },
  });

  map.addLayer({
    id: 'nm-selection-points',
    type: 'circle',
    source: SOURCE_SELECTION_POINTS,
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 9, 11, 12, 14, 15],
      'circle-color': '#f8fafc',
      'circle-opacity': 0.92,
      'circle-stroke-width': 3,
      'circle-stroke-color': '#0f172a',
    },
  });
}
