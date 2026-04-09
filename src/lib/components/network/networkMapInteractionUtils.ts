import type { Geometry, Point } from 'geojson';
import type {
  NMLink,
  NMNode,
  NetworkMapPopupActionModel,
  NetworkMapPopupModel,
} from './networkMapUtils';

function escapePopupHtml(input: unknown): string {
  return String(input ?? '-')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
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
  return {
    buttonId,
    html: `<button id="${buttonId}" class="nm-popup-btn ${toneClass}" type="button">${action.label}</button>`,
  };
}

function renderPopupModel(args: { popupUid: string; model: NetworkMapPopupModel }) {
  const closeBtnId = `${args.popupUid}-close`;
  const actionButtons = args.model.actions.map((action) => ({
    key: action.key,
    ...renderPopupActionButton(args.popupUid, action),
  }));

  return {
    closeBtnId,
    actionButtons,
    html: `
      <div class="nm-popup-card">
        <div class="nm-popup-head">
          <div>
            <div class="nm-popup-kicker">${escapePopupHtml(args.model.kicker)}</div>
            <div class="nm-popup-title">${escapePopupHtml(args.model.title)}</div>
            <div class="nm-popup-subtitle">${escapePopupHtml(args.model.subtitle)}</div>
          </div>
          <span class="nm-popup-badge ${args.model.tone}">${escapePopupHtml(args.model.statusText)}</span>
        </div>
        <div class="nm-popup-impact">${escapePopupHtml(args.model.impactText)}</div>
        <div class="nm-popup-grid">
          ${args.model.detailPairs
            .map(
              (pair) => `
            <div class="nm-popup-label">${escapePopupHtml(pair.label)}</div>
            <div class="nm-popup-value">${escapePopupHtml(pair.value)}</div>
          `,
            )
            .join('')}
        </div>
        <div class="nm-popup-actions">
          ${actionButtons.map((button) => button.html).join('')}
          <button id="${closeBtnId}" class="nm-popup-btn" type="button">Close</button>
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

export function buildRouterPopupHtml(args: {
  popupUid: string;
  name: string;
  tone: 'ok' | 'muted';
  status: string;
  host: string;
  port: string;
  latency: string;
}) {
  const openBtnId = `${args.popupUid}-open`;
  const closeBtnId = `${args.popupUid}-close`;
  return {
    openBtnId,
    closeBtnId,
    html: `
      <div class="nm-popup-card">
        <div class="nm-popup-head">
          <div class="nm-popup-title">${args.name}</div>
          <span class="nm-popup-badge ${args.tone}">${args.status}</span>
        </div>
        <div class="nm-popup-grid">
          <div class="nm-popup-label">Host</div>
          <div class="nm-popup-value mono">${args.host}:${args.port}</div>
          <div class="nm-popup-label">Latency</div>
          <div class="nm-popup-value">${args.latency}</div>
        </div>
        <div class="nm-popup-actions">
          <button id="${openBtnId}" class="nm-popup-btn primary" type="button">Open Router</button>
          <button id="${closeBtnId}" class="nm-popup-btn" type="button">Close</button>
        </div>
      </div>
    `,
  };
}

export function pointCoordinates(geometry: Geometry): [number, number] {
  return (geometry as Point).coordinates as [number, number];
}
