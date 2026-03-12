import type { Point } from 'geojson';

export const INTERACTIVE_LAYER_IDS = [
  'nm-zones-fill',
  'nm-zones-outline',
  'nm-links-line',
  'nm-links-line-dashed',
  'nm-nodes-circle',
  'nm-nodes-icons',
  'nm-routers-circle',
  'nm-routers-icon',
  'nm-customers-cluster-circle',
  'nm-customers-cluster-count',
  'nm-customers-point',
  'nm-link-draft-line',
  'nm-link-draft-points',
] as const;

export function createMapTextButtonControl(args: {
  className: string;
  onClick: () => void;
  onMount?: (button: HTMLButtonElement) => void;
  onUnmount?: () => void;
}) {
  return {
    onAdd: () => {
      const wrap = document.createElement('div');
      wrap.className = 'maplibregl-ctrl maplibregl-ctrl-group';

      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = `maplibregl-ctrl-icon ${args.className}`;
      btn.onclick = args.onClick;
      wrap.appendChild(btn);
      args.onMount?.(btn);
      return wrap;
    },
    onRemove: () => {
      args.onUnmount?.();
    },
  };
}

export function registerInteractiveLayerHover(
  map: import('maplibre-gl').Map,
  layerIds: readonly string[] = INTERACTIVE_LAYER_IDS,
) {
  for (const layerId of layerIds) {
    map.on('mouseenter', layerId, () => {
      map.getCanvas().style.cursor = 'pointer';
    });
    map.on('mouseleave', layerId, () => {
      map.getCanvas().style.cursor = '';
    });
  }
}

export function registerPrimaryLayerClicks(args: {
  map: import('maplibre-gl').Map;
  onNodeClick: (e: any) => void;
  onRouterClick: (e: any) => void;
  onLinkClick: (e: any) => void;
  onCustomerClusterClick: (e: any) => void;
}) {
  args.map.on('click', 'nm-nodes-circle', args.onNodeClick);
  args.map.on('click', 'nm-nodes-icons', args.onNodeClick);
  args.map.on('click', 'nm-routers-circle', args.onRouterClick);
  args.map.on('click', 'nm-routers-icon', args.onRouterClick);
  args.map.on('click', 'nm-customers-point', args.onNodeClick);
  args.map.on('click', 'nm-customers-cluster-circle', args.onCustomerClusterClick);
  args.map.on('click', 'nm-links-line', args.onLinkClick);
  args.map.on('click', 'nm-links-line-dashed', args.onLinkClick);
}

export async function expandCustomerCluster(args: {
  map: import('maplibre-gl').Map;
  feature: any;
  sourceId: string;
}) {
  const clusterId = args.feature.properties?.cluster_id;
  const src = args.map.getSource(args.sourceId) as import('maplibre-gl').GeoJSONSource | undefined;
  if (!src || clusterId == null) return;
  const zoom = await src.getClusterExpansionZoom(clusterId);
  const coords = (args.feature.geometry as Point).coordinates as [number, number];
  args.map.easeTo({ center: coords, zoom: Math.max(zoom, args.map.getZoom() + 1), duration: 280 });
}
