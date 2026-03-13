import { escapeHtml } from './networkMapUtils';

type MaplibreModule = typeof import('maplibre-gl');
type MarkerInstance = import('maplibre-gl').Marker;
type MapInstance = import('maplibre-gl').Map;

export function emitInstallationRefreshSignal(args: {
  fromInstallation: boolean;
  sourceWorkOrderId: string;
}) {
  if (!args.fromInstallation || !args.sourceWorkOrderId) return;
  try {
    localStorage.setItem(
      'nm_installation_work_order_refresh',
      JSON.stringify({
        work_order_id: args.sourceWorkOrderId,
        ts: Date.now(),
      }),
    );
  } catch {
    // Ignore storage errors
  }
}

export function emitWorkOrderUpdatedToParent(args: {
  fromInstallation: boolean;
  sourceWorkOrderId: string;
}) {
  if (!args.fromInstallation || !args.sourceWorkOrderId) return;
  try {
    if (typeof window !== 'undefined' && window.parent && window.parent !== window) {
      window.parent.postMessage(
        {
          type: 'nm_work_order_updated',
          work_order_id: args.sourceWorkOrderId,
          ts: Date.now(),
        },
        window.location.origin,
      );
    }
  } catch {
    // Ignore cross-window messaging errors
  }
}

export async function resolveInstallationTargetMarker(args: {
  map: MapInstance | null;
  maplibre: MaplibreModule | null;
  fromInstallation: boolean;
  sourceCustomerId: string;
  sourceLocationId: string;
  compactMode: boolean;
  didInitialFitToMarkers: boolean;
  existingMarker: MarkerInstance | null;
  loadCustomerLocations: (customerId: string) => Promise<any[]>;
}) {
  if (!args.map || !args.maplibre || !args.fromInstallation) return null;
  if (!args.sourceCustomerId || !args.sourceLocationId) return null;

  const locations = await args.loadCustomerLocations(args.sourceCustomerId);
  const target = (locations || []).find((row) => row.id === args.sourceLocationId);
  if (!target) return null;
  if (!Number.isFinite(target.longitude) || !Number.isFinite(target.latitude)) return null;

  const lng = Number(target.longitude);
  const lat = Number(target.latitude);

  args.existingMarker?.remove();
  const popup = new args.maplibre.Popup({ offset: 10 }).setHTML(
    `<div class="nm-popup"><div class="nm-popup-title">${escapeHtml(target.label || 'Installation Location')}</div><div class="nm-popup-line">${escapeHtml(target.address_line1 || '')}</div></div>`,
  );
  const marker = new args.maplibre.Marker({
    color: '#06b6d4',
    scale: 1.08,
  })
    .setLngLat([lng, lat])
    .setPopup(popup)
    .addTo(args.map);

  // For installation flow, always focus the customer target marker first.
  if (args.compactMode || args.fromInstallation) {
    args.map.easeTo({
      center: [lng, lat],
      zoom: Math.max(args.map.getZoom(), 16),
      duration: 420,
    });
  } else if (!args.didInitialFitToMarkers) {
    args.map.easeTo({
      center: [lng, lat],
      zoom: Math.max(args.map.getZoom(), 14),
      duration: 420,
    });
  }

  return {
    marker,
    coord: [lng, lat] as [number, number],
  };
}
