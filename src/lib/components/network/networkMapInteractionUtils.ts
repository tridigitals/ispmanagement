import type { Geometry, Point } from 'geojson';
import type {
  NMLink,
  NMNode,
  NetworkMapPopupActionModel,
  NetworkMapPopupModel,
} from './networkMapUtils';

export type PopupAnchorPlacement =
  | 'top'
  | 'bottom'
  | 'left'
  | 'right'
  | 'top-left'
  | 'top-right'
  | 'bottom-left'
  | 'bottom-right';

function escapePopupHtml(input: unknown): string {
  return String(input ?? '-')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}

function truncatePopupText(input: unknown, maxLength: number): string {
  const text = String(input ?? '').trim();
  if (!text || text.length <= maxLength) return text;
  return `${text.slice(0, Math.max(0, maxLength - 3)).trimEnd()}...`;
}

function renderPopupTextValue(args: {
  value: unknown;
  className: string;
  truncateTo: number;
}) {
  const full = String(args.value ?? '').trim() || '-';
  const display = truncatePopupText(full, args.truncateTo) || '-';
  const escapedTitle = escapePopupHtml(full);
  const escapedDisplay = escapePopupHtml(display);
  return `<div class="${args.className}" title="${escapedTitle}">${escapedDisplay}</div>`;
}

export function buildDefaultLineGeometry(
  nodeRows: NMNode[],
  fromId: string,
  toId: string,
): Geometry {
  const from = nodeRows.find((x) => x.id === fromId);
  const to = nodeRows.find((x) => x.id === toId);
  if (!from || !to) {
    return {
      type: 'LineString',
      coordinates: [
        [106.84, -6.2],
        [106.87, -6.21],
      ],
    };
  }
  return {
    type: 'LineString',
    coordinates: [
      [from.lng, from.lat],
      [to.lng, to.lat],
    ],
  };
}

export function getNodeCoord(nodeRows: NMNode[], nodeId: string): [number, number] | null {
  const node = nodeRows.find((x) => x.id === nodeId);
  return node ? [node.lng, node.lat] : null;
}

export function currentDraftPathCoords(
  nodeRows: NMNode[],
  linkForm: { from_node_id: string; to_node_id: string },
  linkPathBendPoints: Array<[number, number]>,
  includeToNode = false,
): Array<[number, number]> {
  const coords: Array<[number, number]> = [];
  const fromCoord = linkForm.from_node_id ? getNodeCoord(nodeRows, linkForm.from_node_id) : null;
  if (fromCoord) coords.push(fromCoord);
  if (linkPathBendPoints.length > 0) coords.push(...linkPathBendPoints);
  if (includeToNode && linkForm.to_node_id) {
    const toCoord = getNodeCoord(nodeRows, linkForm.to_node_id);
    if (toCoord) coords.push(toCoord);
  }
  return coords;
}

export function hasExistingLinkBetweenNodes(
  linkRows: NMLink[],
  fromNodeId: string,
  toNodeId: string,
  excludeLinkId?: string | null,
): boolean {
  if (!fromNodeId || !toNodeId || fromNodeId === toNodeId) return false;
  return linkRows.some((row) => {
    if (excludeLinkId && row.id === excludeLinkId) return false;
    return (
      (row.from_node_id === fromNodeId && row.to_node_id === toNodeId) ||
      (row.from_node_id === toNodeId && row.to_node_id === fromNodeId)
    );
  });
}

export function buildDeleteConfirmCopy(
  targetType: 'node' | 'link' | 'zone' | 'binding',
  name?: string,
) {
  const label = name?.trim() ? `"${name.trim()}"` : 'this item';
  if (targetType === 'node') {
    return {
      title: 'Delete Node',
      message: `Delete node ${label}? This action cannot be undone.`,
    };
  }
  if (targetType === 'link') {
    return {
      title: 'Delete Link',
      message: `Delete link ${label}? This action cannot be undone.`,
    };
  }
  if (targetType === 'zone') {
    return {
      title: 'Delete Zone',
      message: `Delete zone ${label}? This action cannot be undone.`,
    };
  }
  return {
    title: 'Delete Binding',
    message: `Delete binding ${label}? This action cannot be undone.`,
  };
}

function renderPopupActionButton(popupUid: string, action: NetworkMapPopupActionModel) {
  const buttonId = `${popupUid}-${action.key}`;
  const toneClass =
    action.tone === 'primary' ? 'primary' : action.tone === 'danger' ? 'danger' : '';
  const actionClass = `action-${action.key}`;
  return {
    buttonId,
    html: `<button id="${buttonId}" class="nm-popup-btn ${toneClass} ${actionClass}" type="button">${action.label}</button>`,
  };
}

function renderPopupModel(args: { popupUid: string; model: NetworkMapPopupModel }) {
  const closeBtnId = `${args.popupUid}-close`;
  const actionButtons = args.model.actions.map((action) => ({
    key: action.key,
    ...renderPopupActionButton(args.popupUid, action),
  }));
  const cardClass =
    args.model.variant === 'workflow-service'
      ? 'nm-popup-card nm-popup-card-workflow'
      : args.model.variant === 'network-link'
        ? 'nm-popup-card nm-popup-card-link'
      : 'nm-popup-card';
  const summaryClass =
    args.model.variant === 'workflow-service'
      ? 'nm-popup-summary nm-popup-summary-workflow'
      : args.model.variant === 'network-link'
        ? 'nm-popup-summary nm-popup-summary-link'
      : 'nm-popup-summary';
  const statusChipsClass =
    args.model.variant === 'workflow-service'
      ? 'nm-popup-status-chips nm-popup-status-chips-workflow'
      : 'nm-popup-status-chips';
  const contextClass =
    args.model.variant === 'workflow-service'
      ? 'nm-popup-context nm-popup-context-workflow'
      : 'nm-popup-context';
  const actionsClass =
    args.model.variant === 'workflow-service'
      ? 'nm-popup-actions nm-popup-actions-workflow'
      : args.model.variant === 'network-link'
        ? 'nm-popup-actions nm-popup-actions-link'
      : 'nm-popup-actions';
  const shouldRenderDetailGrid =
    args.model.variant !== 'network-link' && args.model.detailPairs.length > 0;

  return {
    closeBtnId,
    actionButtons,
    html: `
      <div class="${cardClass}">
        <div class="nm-popup-head">
          <div>
            <div class="nm-popup-kicker">${escapePopupHtml(args.model.kicker)}</div>
            <div class="nm-popup-title">${escapePopupHtml(args.model.title)}</div>
            <div class="nm-popup-subtitle">${escapePopupHtml(args.model.subtitle)}</div>
          </div>
          <span class="nm-popup-badge ${args.model.tone}">${escapePopupHtml(args.model.statusText)}</span>
        </div>
        ${
          args.model.statusChips?.length
            ? `
          <div class="${statusChipsClass}">
            ${args.model.statusChips
              .map(
                (chip) => `
              <div class="nm-popup-status-chip ${chip.tone || 'muted'}">
                <div class="nm-popup-status-chip-label">${escapePopupHtml(chip.label)}</div>
                <div class="nm-popup-status-chip-value">${escapePopupHtml(chip.value)}</div>
              </div>
            `,
              )
              .join('')}
          </div>
        `
            : ''
        }
        <div class="${contextClass}">${escapePopupHtml(args.model.contextText)}</div>
        ${
          args.model.summaryItems.length
            ? `
          <div class="${summaryClass}">
            ${args.model.summaryItems
              .map(
                (item) => `
              <div class="nm-popup-summary-item ${item.tone || 'muted'}">
                <div class="nm-popup-summary-label">${escapePopupHtml(item.label)}</div>
                ${renderPopupTextValue({
                  value: item.value,
                  className: 'nm-popup-summary-value',
                  truncateTo: 30,
                })}
              </div>
            `,
              )
              .join('')}
          </div>
        `
            : ''
        }
        ${
          shouldRenderDetailGrid
            ? `
          <div class="nm-popup-grid">
            ${args.model.detailPairs
              .map(
                (pair) => `
              <div class="nm-popup-label">${escapePopupHtml(pair.label)}</div>
              ${renderPopupTextValue({
                value: pair.value,
                className: 'nm-popup-value',
                truncateTo: 32,
              })}
            `,
              )
              .join('')}
          </div>
        `
            : ''
        }
        <div class="${actionsClass}">
          ${actionButtons.map((button) => button.html).join('')}
          <button id="${closeBtnId}" class="nm-popup-btn nm-popup-btn-close" type="button">Close</button>
        </div>
      </div>
    `,
  };
}

export function buildNodePopupHtml(args: { popupUid: string; model: NetworkMapPopupModel }) {
  return renderPopupModel(args);
}

export function buildLinkPopupHtml(args: { popupUid: string; model: NetworkMapPopupModel }) {
  return renderPopupModel(args);
}

export function buildRouterPopupHtml(args: { popupUid: string; model: NetworkMapPopupModel }) {
  return renderPopupModel(args);
}

export function getPopupSizeForModel(model: Pick<NetworkMapPopupModel, 'variant'>): {
  width: number;
  height: number;
} {
  if (model.variant === 'workflow-service') {
    return { width: 332, height: 320 };
  }
  if (model.variant === 'network-link') {
    return { width: 252, height: 190 };
  }
  return { width: 288, height: 320 };
}

export function pointCoordinates(geometry: Geometry): [number, number] {
  return (geometry as Point).coordinates as [number, number];
}

export function computePopupPlacement(args: {
  point: { x: number; y: number };
  mapSize: { width: number; height: number };
  popupSize: { width: number; height: number };
  padding?: number;
  offset?: number;
}): { anchor: PopupAnchorPlacement; offset: number } {
  const padding = args.padding ?? 16;
  const offset = args.offset ?? 14;
  const spaceLeft = args.point.x - padding;
  const spaceRight = args.mapSize.width - args.point.x - padding;
  const spaceTop = args.point.y - padding;
  const spaceBottom = args.mapSize.height - args.point.y - padding;

  if (spaceRight < args.popupSize.width * 0.65 && spaceLeft > args.popupSize.width * 0.45) {
    return { anchor: 'left', offset };
  }

  if (spaceLeft < args.popupSize.width * 0.45 && spaceRight > args.popupSize.width * 0.55) {
    return { anchor: 'right', offset };
  }

  if (spaceTop < args.popupSize.height * 0.45 && spaceBottom > args.popupSize.height * 0.45) {
    return { anchor: 'bottom', offset };
  }

  if (spaceBottom < args.popupSize.height * 0.35 && spaceTop > args.popupSize.height * 0.45) {
    return { anchor: 'top', offset };
  }

  return { anchor: 'top', offset };
}

export function computePopupViewportNudge(args: {
  popupRect: Pick<DOMRect, 'left' | 'right' | 'top' | 'bottom'>;
  mapRect: Pick<DOMRect, 'left' | 'right' | 'top' | 'bottom'>;
  padding?: number;
}) {
  const padding = args.padding ?? 16;
  let x = 0;
  let y = 0;

  const minLeft = args.mapRect.left + padding;
  const maxRight = args.mapRect.right - padding;
  const minTop = args.mapRect.top + padding;
  const maxBottom = args.mapRect.bottom - padding;

  if (args.popupRect.left < minLeft) {
    x = minLeft - args.popupRect.left;
  } else if (args.popupRect.right > maxRight) {
    x = maxRight - args.popupRect.right;
  }

  if (args.popupRect.top < minTop) {
    y = minTop - args.popupRect.top;
  } else if (args.popupRect.bottom > maxBottom) {
    y = maxBottom - args.popupRect.bottom;
  }

  return { x, y };
}

export function nudgePopupElementIntoView(args: {
  popupElement: HTMLElement | null | undefined;
  mapElement: HTMLElement | null | undefined;
  padding?: number;
}) {
  if (!args.popupElement || !args.mapElement) return;
  const nudge = computePopupViewportNudge({
    popupRect: args.popupElement.getBoundingClientRect(),
    mapRect: args.mapElement.getBoundingClientRect(),
    padding: args.padding,
  });
  const baseTransform =
    args.popupElement.dataset.nmPopupBaseTransform || args.popupElement.style.transform || '';
  args.popupElement.dataset.nmPopupBaseTransform = baseTransform;
  args.popupElement.style.transform =
    nudge.x || nudge.y ? `${baseTransform} translate(${nudge.x}px, ${nudge.y}px)` : baseTransform;
}
