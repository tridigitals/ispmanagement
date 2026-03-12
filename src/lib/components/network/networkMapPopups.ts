import type { Geometry } from 'geojson';
import {
  buildLinkPopupHtml,
  buildNodePopupHtml,
  buildRouterPopupHtml,
  pointCoordinates,
} from './networkMapInteractionUtils';
import {
  computeLinkHealth,
  escapeHtml,
  isSystemManagedNode,
  nodeTypeLabel,
  statusTone,
  systemManagedNodeSourceLabel,
  type NMLink,
  type NMNode,
} from './networkMapUtils';

type PopupInstance = import('maplibre-gl').Popup;
type PopupFactory = import('maplibre-gl').Popup;

export function openNodePopup(args: {
  map: import('maplibre-gl').Map;
  maplibre: { Popup: PopupFactory };
  feature: { properties?: Record<string, any>; geometry: Geometry };
  nodeRows: NMNode[];
  activePopup: PopupInstance | null;
  setActivePopup: (popup: PopupInstance | null) => void;
  onConnect: (nodeId: string) => void;
  onEdit: (node: NMNode) => void;
}) {
  const props = args.feature.properties || {};
  const coords = pointCoordinates(args.feature.geometry);
  const nodeId = String(props.id || '');
  const node = args.nodeRows.find((x) => x.id === nodeId);
  const managed = isSystemManagedNode(node);
  const managedSource = systemManagedNodeSourceLabel(node);
  const popupUid = `nm-popup-${Math.random().toString(36).slice(2, 10)}`;
  const status = escapeHtml(props.status || '-');
  const tone = statusTone(props.status);
  const name = escapeHtml(props.name || '-');
  const nodeType = escapeHtml(nodeTypeLabel(props.node_type));
  const sourceDetail = managedSource
    ? `<div class="nm-popup-label">Source</div><div class="nm-popup-value">${escapeHtml(managedSource)}</div>`
    : '';
  const popupContent = buildNodePopupHtml({
    popupUid,
    name,
    tone,
    status,
    nodeType,
    sourceDetailHtml: sourceDetail,
    managed,
  });

  args.activePopup?.remove();
  const popup = new args.maplibre.Popup({ closeButton: false, closeOnClick: true })
    .setLngLat(coords as [number, number])
    .setHTML(popupContent.html);

  popup.on('open', () => {
    const connectBtn = document.getElementById(popupContent.connectBtnId) as HTMLButtonElement | null;
    const editBtn = document.getElementById(popupContent.editBtnId) as HTMLButtonElement | null;
    const closeBtn = document.getElementById(popupContent.closeBtnId) as HTMLButtonElement | null;

    connectBtn?.addEventListener('click', () => {
      popup.remove();
      args.onConnect(nodeId);
    });
    editBtn?.addEventListener('click', () => {
      popup.remove();
      if (node) args.onEdit(node);
    });
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
  maplibre: { Popup: PopupFactory };
  feature: { properties?: Record<string, any> };
  lngLat: { lng: number; lat: number };
  linkRows: NMLink[];
  onDelete: (linkId: string, linkName?: string) => void;
}) {
  const props = args.feature.properties || {};
  const linkId = String(props.id || '');
  const link = args.linkRows.find((x) => x.id === linkId);
  if (!link) return;

  const popupUid = `nm-link-popup-${Math.random().toString(36).slice(2, 10)}`;
  const name = escapeHtml(link.name || '-');
  const status = escapeHtml(link.status || '-');
  const type = escapeHtml(link.link_type || '-');
  const health = computeLinkHealth(link);
  const popupContent = buildLinkPopupHtml({
    popupUid,
    name,
    healthTone: health.tone,
    healthScore: health.score,
    type,
    status,
    endpoints: `${escapeHtml(link.from_node_id || '-')} -> ${escapeHtml(link.to_node_id || '-')}`,
  });
  const popup = new args.maplibre.Popup({ closeButton: false, closeOnClick: true })
    .setLngLat([args.lngLat.lng, args.lngLat.lat])
    .setHTML(popupContent.html);

  popup.on('open', () => {
    const deleteBtn = document.getElementById(popupContent.deleteBtnId) as HTMLButtonElement | null;
    const closeBtn = document.getElementById(popupContent.closeBtnId) as HTMLButtonElement | null;
    deleteBtn?.addEventListener('click', () => {
      popup.remove();
      args.onDelete(linkId, link.name);
    });
    closeBtn?.addEventListener('click', () => popup.remove());
  });
  popup.addTo(args.map);
}

export function openRouterPopup(args: {
  map: import('maplibre-gl').Map;
  maplibre: { Popup: PopupFactory };
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
