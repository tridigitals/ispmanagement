import type { NMNode, NMRouter } from './networkMapUtils';

export function fitMapToMarkers(args: {
  map: import('maplibre-gl').Map;
  maplibre: typeof import('maplibre-gl');
  didInitialFitToMarkers: boolean;
  nodes: NMNode[];
  routers: NMRouter[];
  installationTargetCoord: [number, number] | null;
}) {
  if (args.didInitialFitToMarkers) return false;
  const points: Array<[number, number]> = [];

  for (const row of args.nodes || []) {
    if (Number.isFinite(row.lng) && Number.isFinite(row.lat)) {
      points.push([row.lng, row.lat]);
    }
  }

  for (const row of args.routers || []) {
    if (row.longitude == null || row.latitude == null) continue;
    if (Number.isFinite(row.longitude) && Number.isFinite(row.latitude)) {
      points.push([row.longitude, row.latitude]);
    }
  }

  if (args.installationTargetCoord) {
    points.push(args.installationTargetCoord);
  }
  if (!points.length) return false;

  const bounds = points.reduce(
    (acc, point) => acc.extend(point),
    new args.maplibre.LngLatBounds(points[0], points[0]),
  );
  args.map.fitBounds(bounds, {
    padding: { top: 80, right: 80, bottom: 80, left: 80 },
    maxZoom: 15,
    duration: 600,
  });
  return true;
}

export async function showMyLocationMarker(args: {
  map: import('maplibre-gl').Map;
  maplibre: typeof import('maplibre-gl');
  existingMarker: import('maplibre-gl').Marker | null;
}) {
  if (!navigator.geolocation) {
    throw new Error('Geolocation is not supported by this browser.');
  }

  const pos = await new Promise<GeolocationPosition>((resolve, reject) => {
    navigator.geolocation.getCurrentPosition(resolve, reject, {
      enableHighAccuracy: true,
      timeout: 12000,
      maximumAge: 30000,
    });
  });

  const lng = pos.coords.longitude;
  const lat = pos.coords.latitude;
  let marker = args.existingMarker;

  if (!marker) {
    const el = document.createElement('div');
    el.className = 'my-location-dot';
    marker = new args.maplibre.Marker({ element: el, anchor: 'center' });
  }

  marker.setLngLat([lng, lat]).addTo(args.map);
  args.map.flyTo({ center: [lng, lat], zoom: Math.max(args.map.getZoom(), 15), speed: 1.1 });
  return marker;
}

export function hideMyLocationMarker(marker: import('maplibre-gl').Marker | null) {
  marker?.remove();
}

export function syncMyLocationControlButton(args: {
  button: HTMLButtonElement | null;
  label: string;
  locating: boolean;
  mapUnavailable: boolean;
  myLocationVisible: boolean;
}) {
  const btn = args.button;
  if (!btn) return;
  btn.disabled = args.locating || args.mapUnavailable;
  btn.title = args.label;
  btn.setAttribute('aria-label', args.label);
  btn.textContent = args.locating ? '↻' : '◎';
  btn.classList.toggle('active', args.myLocationVisible);
  btn.classList.toggle('loading', args.locating);
}

export function syncViewModeControlButton(args: {
  button: HTMLButtonElement | null;
  isSatellite: boolean;
}) {
  const btn = args.button;
  if (!btn) return;
  const label = args.isSatellite ? 'Switch to standard map' : 'Switch to satellite map';
  btn.title = label;
  btn.setAttribute('aria-label', label);
  btn.textContent = args.isSatellite ? 'SAT' : 'MAP';
  btn.classList.toggle('active', args.isSatellite);
}
