import type { FeatureCollection } from 'geojson';

import { handleCanvasMapClick } from './networkMapCanvasInteractions';
import {
  expandCustomerCluster,
  registerInteractiveLayerHover,
  registerPrimaryLayerClicks,
} from './networkMapInit';
import {
  buildBaseMapStyle,
  emptyFeatureCollection,
  registerMapSourcesAndLayers,
} from './networkMapLayers';
import { fitMapToMarkers } from './networkMapRuntime';
import type { NetworkMapWorkspaceSelectedObject } from './networkMapWorkspaceState';
import {
  ensureNodeTypeIconsRegistered,
  linksToFeatureCollection,
  nodesToFeatureCollection,
  routersToFeatureCollection,
  type NMLink,
  type NMNode,
  type NMRouter,
  type NMZone,
  zonesToFeatureCollection,
} from './networkMapUtils';

export { buildBaseMapStyle };
export { ensureNodeTypeIconsRegistered };
export { expandCustomerCluster };
export { fitMapToMarkers };
export { handleCanvasMapClick };
export { registerInteractiveLayerHover };
export { registerMapSourcesAndLayers };
export { registerPrimaryLayerClicks };

export function buildSelectionFeatureCollections(args: {
  selectedObject: NetworkMapWorkspaceSelectedObject | null;
  nodeRows: NMNode[];
  linkRows: NMLink[];
  zoneRows: NMZone[];
  routerRows: NMRouter[];
}): {
  pointData: FeatureCollection;
  lineData: FeatureCollection;
  zoneData: FeatureCollection;
} {
  let pointData = emptyFeatureCollection();
  let lineData = emptyFeatureCollection();
  let zoneData = emptyFeatureCollection();

  if (!args.selectedObject) {
    return { pointData, lineData, zoneData };
  }

  if (
    args.selectedObject.kind === 'node' ||
    args.selectedObject.kind === 'customer' ||
    args.selectedObject.kind === 'service'
  ) {
    const row = args.nodeRows.find((candidate) => candidate.id === args.selectedObject?.id);
    if (row) pointData = nodesToFeatureCollection([row]);
    return { pointData, lineData, zoneData };
  }

  if (args.selectedObject.kind === 'link') {
    const row = args.linkRows.find((candidate) => candidate.id === args.selectedObject?.id);
    if (row) lineData = linksToFeatureCollection([row]);
    return { pointData, lineData, zoneData };
  }

  if (args.selectedObject.kind === 'zone') {
    const row = args.zoneRows.find((candidate) => candidate.id === args.selectedObject?.id);
    if (row) zoneData = zonesToFeatureCollection([row]);
    return { pointData, lineData, zoneData };
  }

  if (args.selectedObject.kind === 'router') {
    const row = args.routerRows.find((candidate) => candidate.id === args.selectedObject?.id);
    if (row) pointData = routersToFeatureCollection([row]);
  }

  return { pointData, lineData, zoneData };
}
