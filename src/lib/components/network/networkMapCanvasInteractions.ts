import type { NMNode } from './networkMapUtils';

const NODE_HIT_LAYERS = [
  'nm-nodes-circle',
  'nm-nodes-icons',
  'nm-routers-circle',
  'nm-routers-icon',
  'nm-customers-point',
] as const;

export function isExistingNodeHit(
  map: import('maplibre-gl').Map,
  point: import('maplibre-gl').PointLike,
) {
  return (
    map.queryRenderedFeatures(point, {
      layers: [...NODE_HIT_LAYERS],
    }).length > 0
  );
}

export function snapLinkPointToNearestNode(args: {
  map: import('maplibre-gl').Map;
  linkSnapToNodeEnabled: boolean;
  nodeRows: NMNode[];
  lng: number;
  lat: number;
  maxDistancePx?: number;
}): { lng: number; lat: number; nodeId: string; nodeName: string } | null {
  if (!args.linkSnapToNodeEnabled || !args.nodeRows.length) return null;
  const clickPoint = args.map.project([args.lng, args.lat]);
  let best:
    | {
        row: NMNode;
        distance: number;
      }
    | null = null;

  for (const row of args.nodeRows) {
    if (!Number.isFinite(row.lng) || !Number.isFinite(row.lat)) continue;
    const point = args.map.project([row.lng, row.lat]);
    const dx = point.x - clickPoint.x;
    const dy = point.y - clickPoint.y;
    const distance = Math.sqrt(dx * dx + dy * dy);
    if (!best || distance < best.distance) {
      best = { row, distance };
    }
  }

  const maxDistancePx = args.maxDistancePx ?? 16;
  if (!best || best.distance > maxDistancePx) return null;
  return {
    lng: best.row.lng,
    lat: best.row.lat,
    nodeId: best.row.id,
    nodeName: best.row.name || 'Node',
  };
}

export function handleCanvasMapClick(args: {
  map: import('maplibre-gl').Map;
  event: any;
  linkPickMode: boolean;
  linkPickDrawMode: 'quick' | 'path';
  linkForm: { from_node_id: string };
  linkSnapToNodeEnabled: boolean;
  nodeRows: NMNode[];
  nodePickMode: boolean;
  onAddLinkPathPoint: (point: [number, number]) => void;
  onApplyPickedNodeCoordinates: (lng: number, lat: number) => void;
}) {
  if (args.linkPickMode && args.linkPickDrawMode === 'path' && args.linkForm.from_node_id) {
    const hitNode = isExistingNodeHit(args.map, args.event.point);
    if (!hitNode) {
      const snapped = snapLinkPointToNearestNode({
        map: args.map,
        linkSnapToNodeEnabled: args.linkSnapToNodeEnabled,
        nodeRows: args.nodeRows,
        lng: args.event.lngLat.lng,
        lat: args.event.lngLat.lat,
      });
      const nextPoint: [number, number] = snapped
        ? [snapped.lng, snapped.lat]
        : [args.event.lngLat.lng, args.event.lngLat.lat];
      args.onAddLinkPathPoint(nextPoint);
      return { handled: true };
    }
  }

  if (!args.nodePickMode) return { handled: false };
  const hitNode = isExistingNodeHit(args.map, args.event.point);
  if (hitNode) return { handled: true };
  args.onApplyPickedNodeCoordinates(args.event.lngLat.lng, args.event.lngLat.lat);
  return { handled: true };
}
