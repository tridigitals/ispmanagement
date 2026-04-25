import type { Geometry } from 'geojson';
import {
  buildLinkPopupHtml,
  buildNodePopupHtml,
  buildRouterPopupHtml,
  computePopupPlacement,
  getPopupSizeForModel,
  nudgePopupElementIntoView,
  pointCoordinates,
} from './networkMapInteractionUtils';
import {
  buildLinkPopupModel,
  buildNodePopupModel,
  buildRouterPopupModel,
  buildRouterPopupModelFromNode,
  findLiveRouterForNode,
  buildServicePopupModel,
  type NMLink,
  type NMNode,
  type NMRouter,
} from './networkMapUtils';

type PopupInstance = import('maplibre-gl').Popup;
type MaplibreLike = Pick<typeof import('maplibre-gl'), 'Popup'>;
type PopupDismissEvent = 'movestart' | 'zoomstart' | 'dragstart';

function popupOptionsForMap(
  map: import('maplibre-gl').Map,
  coords: [number, number],
  popupSize: { width: number; height: number } = { width: 288, height: 320 },
) {
  const container = map.getContainer();
  const projected = map.project(coords);
  const placement = computePopupPlacement({
    point: { x: projected.x, y: projected.y },
    mapSize: { width: container.clientWidth, height: container.clientHeight },
    popupSize,
    padding: 18,
    offset: 14,
  });

  return {
    closeButton: false,
    closeOnClick: true,
    anchor: placement.anchor,
    offset: placement.offset,
  };
}

export function bindPopupNavigationDismiss(args: {
  map: Pick<import('maplibre-gl').Map, 'on' | 'off'>;
  popup: { remove: () => void };
}) {
  const events: PopupDismissEvent[] = ['movestart', 'zoomstart', 'dragstart'];
  const dismiss = () => args.popup.remove();
  for (const event of events) args.map.on(event, dismiss);
  return () => {
    for (const event of events) args.map.off(event, dismiss);
  };
}

export function openNodePopup(args: {
  map: import('maplibre-gl').Map;
  maplibre: MaplibreLike;
  feature: { properties?: Record<string, any>; geometry: Geometry };
  nodeRows: NMNode[];
  routerRows: NMRouter[];
  activePopup: PopupInstance | null;
  setActivePopup: (popup: PopupInstance | null) => void;
  onClose?: () => void;
  onOpenCustomer: (customerId: string) => void;
  onOpenService: (customerId: string, serviceId: string) => void;
  onConnect: (nodeId: string) => void;
  onEdit: (node: NMNode) => void;
  onOpenRouter: (routerId: string) => void;
}) {
  const props = args.feature.properties || {};
  const coords = pointCoordinates(args.feature.geometry);
  const nodeId = String(props.id || '');
  const node = args.nodeRows.find((x) => x.id === nodeId);
  const popupUid = `nm-popup-${Math.random().toString(36).slice(2, 10)}`;
  if (!node) return;
  const customerId = String(node.metadata?.customer_id || '').trim();
  const serviceId = String(node.metadata?.service_id || node.metadata?.subscription_id || '').trim();
  const liveRouter = findLiveRouterForNode(node, args.routerRows);
  const popupModel = node.metadata?.service_id
    ? buildServicePopupModel(node)
    : liveRouter
      ? buildRouterPopupModelFromNode(node, liveRouter)
      : buildNodePopupModel(node);
  const popupContent = buildNodePopupHtml({ popupUid, model: popupModel });
  const popupSize = getPopupSizeForModel(popupModel);

  args.activePopup?.remove();
  const popup = new args.maplibre.Popup(
    popupOptionsForMap(args.map, coords as [number, number], popupSize),
  )
    .setLngLat(coords as [number, number])
    .setHTML(popupContent.html);
  let cleanupNavigationDismiss: (() => void) | null = null;

  popup.on('open', () => {
    const popupElement =
      typeof (popup as any).getElement === 'function' ? ((popup as any).getElement() as HTMLElement) : null;
    if (popupModel.variant === 'workflow-service') {
      popupElement?.classList.add('nm-popup-workflow-shell');
    }
    requestAnimationFrame(() => {
      nudgePopupElementIntoView({
        popupElement,
        mapElement: args.map.getContainer(),
        padding: 18,
      });
    });
    cleanupNavigationDismiss = bindPopupNavigationDismiss({
      map: args.map,
      popup,
    });
    const closeBtn = document.getElementById(popupContent.closeBtnId) as HTMLButtonElement | null;
    for (const actionButton of popupContent.actionButtons) {
      const button = document.getElementById(actionButton.buttonId) as HTMLButtonElement | null;
      button?.addEventListener('click', () => {
        popup.remove();
        if (actionButton.key === 'open-customer' && customerId) args.onOpenCustomer(customerId);
        if (actionButton.key === 'open-service' && customerId && serviceId) {
          args.onOpenService(customerId, serviceId);
        }
        if (actionButton.key === 'connect') args.onConnect(nodeId);
        if (actionButton.key === 'edit') args.onEdit(node);
        if (actionButton.key === 'open-router' && liveRouter) args.onOpenRouter(liveRouter.id);
      });
    }
    closeBtn?.addEventListener('click', () => {
      popup.remove();
    });
  });
  popup.on('close', () => {
    cleanupNavigationDismiss?.();
    cleanupNavigationDismiss = null;
    args.setActivePopup(null);
    args.onClose?.();
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
  onClose?: () => void;
  onEdit: (link: NMLink) => void;
  onDelete: (linkId: string, linkName?: string) => void;
}) {
  const props = args.feature.properties || {};
  const linkId = String(props.id || '');
  const link = args.linkRows.find((x) => x.id === linkId);
  if (!link) return;

  const popupUid = `nm-link-popup-${Math.random().toString(36).slice(2, 10)}`;
  const popupModel = buildLinkPopupModel(link);
  const popupContent = buildLinkPopupHtml({ popupUid, model: popupModel });
  const popup = new args.maplibre.Popup(
    popupOptionsForMap(
      args.map,
      [args.lngLat.lng, args.lngLat.lat],
      getPopupSizeForModel(popupModel),
    ),
  )
    .setLngLat([args.lngLat.lng, args.lngLat.lat])
    .setHTML(popupContent.html);
  let cleanupNavigationDismiss: (() => void) | null = null;

  popup.on('open', () => {
    requestAnimationFrame(() => {
      const popupElement =
        typeof (popup as any).getElement === 'function' ? ((popup as any).getElement() as HTMLElement) : null;
      popupElement?.classList.add('nm-popup-link-shell');
      nudgePopupElementIntoView({
        popupElement,
        mapElement: args.map.getContainer(),
        padding: 18,
      });
    });
    cleanupNavigationDismiss = bindPopupNavigationDismiss({
      map: args.map,
      popup,
    });
    const closeBtn = document.getElementById(popupContent.closeBtnId) as HTMLButtonElement | null;
    for (const actionButton of popupContent.actionButtons) {
      const button = document.getElementById(actionButton.buttonId) as HTMLButtonElement | null;
      button?.addEventListener('click', () => {
        popup.remove();
        if (actionButton.key === 'edit') args.onEdit(link);
        if (actionButton.key === 'delete') args.onDelete(linkId, link.name);
      });
    }
    closeBtn?.addEventListener('click', () => popup.remove());
  });
  popup.on('close', () => {
    cleanupNavigationDismiss?.();
    cleanupNavigationDismiss = null;
    args.onClose?.();
  });
  popup.addTo(args.map);
}

export function openRouterPopup(args: {
  map: import('maplibre-gl').Map;
  maplibre: MaplibreLike;
  feature: { properties?: Record<string, any>; geometry: Geometry };
  activePopup: PopupInstance | null;
  setActivePopup: (popup: PopupInstance | null) => void;
  onClose?: () => void;
  onOpenRouter: (routerId: string) => void;
}) {
  const props = args.feature.properties || {};
  const coords = pointCoordinates(args.feature.geometry);
  const routerId = String(props.id || '');
  const model = buildRouterPopupModel({
    id: routerId,
    name: String(props.name || ''),
    identity: String(props.identity || ''),
    host: String(props.host || ''),
    port: Number(props.port || 0),
    is_online: Boolean(props.is_online),
    enabled: Boolean(props.enabled ?? true),
    ros_version: props.ros_version != null ? String(props.ros_version) : null,
    latency_ms: props.latency_ms != null ? Number(props.latency_ms) : null,
  });
  const popupUid = `nm-router-popup-${Math.random().toString(36).slice(2, 10)}`;
  const popupContent = buildRouterPopupHtml({ popupUid, model });

  args.activePopup?.remove();
  const popup = new args.maplibre.Popup(popupOptionsForMap(args.map, coords as [number, number]))
    .setLngLat(coords as [number, number])
    .setHTML(popupContent.html);
  let cleanupNavigationDismiss: (() => void) | null = null;

  popup.on('open', () => {
    requestAnimationFrame(() => {
      nudgePopupElementIntoView({
        popupElement:
          typeof (popup as any).getElement === 'function' ? ((popup as any).getElement() as HTMLElement) : null,
        mapElement: args.map.getContainer(),
        padding: 18,
      });
    });
    cleanupNavigationDismiss = bindPopupNavigationDismiss({
      map: args.map,
      popup,
    });
    const openBtnMeta = popupContent.actionButtons.find((button) => button.key === 'open-router');
    const openBtn = openBtnMeta
      ? (document.getElementById(openBtnMeta.buttonId) as HTMLButtonElement | null)
      : null;
    const closeBtn = document.getElementById(popupContent.closeBtnId) as HTMLButtonElement | null;
    openBtn?.addEventListener('click', () => {
      popup.remove();
      args.onOpenRouter(routerId);
    });
    closeBtn?.addEventListener('click', () => popup.remove());
  });
  popup.on('close', () => {
    cleanupNavigationDismiss?.();
    cleanupNavigationDismiss = null;
    args.setActivePopup(null);
    args.onClose?.();
  });
  args.setActivePopup(popup);
  popup.addTo(args.map);
}
