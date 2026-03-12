import type { FeatureCollection, LineString, Polygon } from 'geojson';
import { buildDefaultLineGeometry, currentDraftPathCoords } from './networkMapInteractionUtils';
import { emptyFeatureCollection } from './networkMapLayers';
import { prettyGeometry, type NMNode } from './networkMapUtils';
import type { NetworkMapLinkForm } from './networkMapLinkPicking';

export function buildDefaultZoneGeometry(map: import('maplibre-gl').Map | null): Polygon {
  if (map) {
    const b = map.getBounds();
    const w = b.getWest();
    const s = b.getSouth();
    const e = b.getEast();
    const n = b.getNorth();
    const padLng = (e - w) * 0.2;
    const padLat = (n - s) * 0.2;
    return {
      type: 'Polygon',
      coordinates: [
        [
          [w + padLng, s + padLat],
          [w + padLng, n - padLat],
          [e - padLng, n - padLat],
          [e - padLng, s + padLat],
          [w + padLng, s + padLat],
        ],
      ],
    };
  }

  return {
    type: 'Polygon',
    coordinates: [
      [
        [106.81, -6.24],
        [106.81, -6.17],
        [106.92, -6.17],
        [106.92, -6.24],
        [106.81, -6.24],
      ],
    ],
  };
}

export function buildLinkGeometryDraftText(args: {
  linkPickDrawMode: 'quick' | 'path';
  nodeRows: NMNode[];
  linkForm: NetworkMapLinkForm;
  linkPathBendPoints: Array<[number, number]>;
}) {
  const coords =
    args.linkPickDrawMode === 'path'
      ? currentDraftPathCoords(
          args.nodeRows,
          args.linkForm,
          args.linkPathBendPoints,
          Boolean(args.linkForm.to_node_id),
        )
      : ((buildDefaultLineGeometry(
          args.nodeRows,
          args.linkForm.from_node_id,
          args.linkForm.to_node_id,
        ) as LineString).coordinates as Array<[number, number]>);

  if (coords.length < 2) return args.linkForm.geometryText;
  return prettyGeometry({ type: 'LineString', coordinates: coords });
}

export function buildLinkDraftPreviewCollections(args: {
  linkPickMode: boolean;
  linkPickDrawMode: 'quick' | 'path';
  nodeRows: NMNode[];
  linkForm: NetworkMapLinkForm;
  linkPathBendPoints: Array<[number, number]>;
}) {
  const lineFc: FeatureCollection = emptyFeatureCollection();
  const pointsFc: FeatureCollection = emptyFeatureCollection();

  if (!args.linkPickMode) {
    return { lineFc, pointsFc };
  }

  const lineCoords =
    args.linkPickDrawMode === 'path'
      ? currentDraftPathCoords(
          args.nodeRows,
          args.linkForm,
          args.linkPathBendPoints,
          false,
        )
      : currentDraftPathCoords(
          args.nodeRows,
          args.linkForm,
          args.linkPathBendPoints,
          Boolean(args.linkForm.to_node_id),
        );

  if (lineCoords.length >= 2) {
    lineFc.features.push({
      type: 'Feature',
      geometry: { type: 'LineString', coordinates: lineCoords },
      properties: {},
    });
  }

  for (const p of lineCoords) {
    pointsFc.features.push({
      type: 'Feature',
      geometry: { type: 'Point', coordinates: p },
      properties: {},
    });
  }

  return { lineFc, pointsFc };
}

export function clearDraftNodeMarker(
  marker: import('maplibre-gl').Marker | null,
  removeMarker: boolean,
) {
  if (removeMarker) {
    marker?.remove();
    return null;
  }
  return marker;
}

export function applyPickedNodeMarker(args: {
  map: import('maplibre-gl').Map;
  maplibre: typeof import('maplibre-gl');
  marker: import('maplibre-gl').Marker | null;
  lng: number;
  lat: number;
  onDrag: (lng: number, lat: number) => void;
}) {
  let marker = args.marker;
  if (!marker) {
    marker = new args.maplibre.Marker({ color: '#3f8cff', draggable: true })
      .setLngLat([args.lng, args.lat])
      .addTo(args.map);
    marker.on('dragend', () => {
      const p = marker?.getLngLat();
      if (!p) return;
      args.onDrag(p.lng, p.lat);
    });
    return marker;
  }

  marker.setLngLat([args.lng, args.lat]);
  return marker;
}
