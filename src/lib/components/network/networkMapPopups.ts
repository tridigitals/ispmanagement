import type { Geometry } from 'geojson';
import {
  buildLinkPopupHtml,
  buildNodePopupHtml,
  buildRouterPopupHtml,
  pointCoordinates,
} from './networkMapInteractionUtils';
import {
  computeLinkHealth,
  buildLinkPopupModel,
  buildNodePopupModel,
  buildServicePopupModel,
  escapeHtml,
  statusTone,
  type NMLink,
  type NMNode,
} from './networkMapUtils';

type PopupInstance = import('maplibre-gl').Popup;
type MaplibreLike = Pick<typeof import('maplibre-gl'), 'Popup'>;

export function openNodePopup(args: {
  map: import('maplibre-gl').Map;
  maplibre: MaplibreLike;
  feature: { properties?: Record<string, any>; geometry: Geometry };
  nodeRows: NMNode[];
  activePopup: PopupInstance | null;
  setActivePopup: (popup: PopupInstance | null) => void;
  onConnect: (nodeId: string) => void;
  onEdit: (node: NMNode) => void;
  onTrace?: (node: NMNode) => void;
  onInspect?: (node: NMNode) => void;
  onViewImpact?: (node: NMNode) => void;
}) {
  const props = args.feature.properties || {};
  const coords = pointCoordinates(args.feature.geometry);
  const nodeId = String(props.id || '');
  const node = args.nodeRows.find((x) => x.id === nodeId);
  const popupUid = `nm-popup-${Math.random().toString(36).slice(2, 10)}`;
  if (!node) return;
  const popupModel = node.metadata?.service_id
    ? buildServicePopupModel(node)
    : buildNodePopupModel(node);
  const popupContent = buildNodePopupHtml({ popupUid, model: popupModel });

  args.activePopup?.remove();
  const popup = new args.maplibre.Popup({ closeButton: false, closeOnClick: true })
    .setLngLat(coords as [number, number])
    .setHTML(popupContent.html);

  popup.on('open', () => {
    const closeBtn = document.getElementById(popupContent.closeBtnId) as HTMLButtonElement | null;
    for (const actionButton of popupContent.actionButtons) {
      const button = document.getElementById(actionButton.buttonId) as HTMLButtonElement | null;
      button?.addEventListener('click', () => {
        popup.remove();
        if (actionButton.key === 'connect') args.onConnect(nodeId);
        if (actionButton.key === 'edit') args.onEdit(node);
        if (actionButton.key === 'trace') args.onTrace?.(node);
        if (actionButton.key === 'inspect') args.onInspect?.(node);
        if (actionButton.key === 'impact') args.onViewImpact?.(node);
      });
    }
    closeBtn?.addEventListener('click', () => {
      popup.remove();
    });
  });
  popup.on('close', () => {
    args.setActivePopup(null);
  });
  args.setActivePopup(popup);
  popup.addTo(args.map);
}

export function openLinkPopup(args: {
  map: import('maplibre-gl').Map;
  maplibre: MaplibreLike;
  feature: { properties?: Record<string, any> };
  lngLat: { lng: number; lat: number };
  linkRows: NMLink[];
  onDelete: (linkId: string, linkName?: string) => void;
  onTrace?: (link: NMLink) => void;
  onInspect?: (link: NMLink) => void;
}) {
  const props = args.feature.properties || {};
  const linkId = String(props.id || '');
  const link = args.linkRows.find((x) => x.id === linkId);
  if (!link) return;

  const popupUid = `nm-link-popup-${Math.random().toString(36).slice(2, 10)}`;
  const popupContent = buildLinkPopupHtml({ popupUid, model: buildLinkPopupModel(link) });
  const popup = new args.maplibre.Popup({ closeButton: false, closeOnClick: true })
    .setLngLat([args.lngLat.lng, args.lngLat.lat])
    .setHTML(popupContent.html);

  popup.on('open', () => {
    const closeBtn = document.getElementById(popupContent.closeBtnId) as HTMLButtonElement | null;
    for (const actionButton of popupContent.actionButtons) {
      const button = document.getElementById(actionButton.buttonId) as HTMLButtonElement | null;
      button?.addEventListener('click', () => {
        popup.remove();
        if (actionButton.key === 'delete') args.onDelete(linkId, link.name);
        if (actionButton.key === 'trace') args.onTrace?.(link);
        if (actionButton.key === 'inspect') args.onInspect?.(link);
      });
    }
    closeBtn?.addEventListener('click', () => popup.remove());
  });
  popup.addTo(args.map);
}

export function openRouterPopup(args: {
  map: import('maplibre-gl').Map;
  maplibre: MaplibreLike;
  feature: { properties?: Record<string, any>; geometry: Geometry };
  activePopup: PopupInstance | null;
  setActivePopup: (popup: PopupInstance | null) => void;
  onOpenRouter: (routerId: string) => void;
}) {
  const props = args.feature.properties || {};
  const coords = pointCoordinates(args.feature.geometry);
  const routerId = String(props.id || '');
  const status = props.is_online ? 'online' : 'offline';
  const tone: 'ok' | 'muted' = props.is_online ? 'ok' : 'muted';
  const name = escapeHtml(props.name || '-');
  const host = escapeHtml(props.host || '-');
  const port = escapeHtml(props.port || '-');
  const latency = props.latency_ms != null ? `${escapeHtml(props.latency_ms)} ms` : '-';
  const popupUid = `nm-router-popup-${Math.random().toString(36).slice(2, 10)}`;
  const popupContent = buildRouterPopupHtml({
    popupUid,
    name,
    tone,
    status,
    host,
    port,
    latency,
  });

  args.activePopup?.remove();
  const popup = new args.maplibre.Popup({ closeButton: false, closeOnClick: true })
    .setLngLat(coords as [number, number])
    .setHTML(popupContent.html);

  popup.on('open', () => {
    const openBtn = document.getElementById(popupContent.openBtnId) as HTMLButtonElement | null;
    const closeBtn = document.getElementById(popupContent.closeBtnId) as HTMLButtonElement | null;
    openBtn?.addEventListener('click', () => {
      popup.remove();
      args.onOpenRouter(routerId);
    });
    closeBtn?.addEventListener('click', () => popup.remove());
  });
  popup.on('close', () => {
    args.setActivePopup(null);
  });
  args.setActivePopup(popup);
  popup.addTo(args.map);
}
