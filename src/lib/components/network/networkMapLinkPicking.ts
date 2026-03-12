import type { Geometry, LineString } from 'geojson';
import { buildLinkDraftForm } from './networkMapActions';
import { buildDefaultLineGeometry, hasExistingLinkBetweenNodes } from './networkMapInteractionUtils';
import { prettyGeometry, type NMLink, type NMNode } from './networkMapUtils';

export type LinkPickStep = 'from' | 'to';
export type LinkPickDrawMode = 'quick' | 'path';

export type NetworkMapLinkForm = {
  name: string;
  link_type: string;
  status: string;
  from_node_id: string;
  to_node_id: string;
  priority: string;
  capacity_mbps: string;
  utilization_pct: string;
  loss_db: string;
  latency_ms: string;
  geometryText: string;
};

export function createLinkForm(nodeRows: NMNode[]): NetworkMapLinkForm {
  const fromNodeId = nodeRows[0]?.id || '';
  const toNodeId = nodeRows[1]?.id || nodeRows[0]?.id || '';
  return {
    name: '',
    link_type: 'fiber',
    status: 'up',
    from_node_id: fromNodeId,
    to_node_id: toNodeId,
    priority: '100',
    capacity_mbps: '',
    utilization_pct: '',
    loss_db: '',
    latency_ms: '',
    geometryText: prettyGeometry(buildDefaultLineGeometry(nodeRows, fromNodeId, toNodeId)),
  };
}

export function createEditLinkForm(row: NMLink, nodeRows: NMNode[]): NetworkMapLinkForm {
  const draft = buildLinkDraftForm(row, buildDefaultLineGeometry(nodeRows, '', ''));
  return {
    ...draft,
    geometryText: prettyGeometry(draft.geometry as Geometry),
  };
}

export function createEmptyLineGeometryText() {
  return prettyGeometry({ type: 'LineString', coordinates: [] } as LineString);
}

export function buildStraightLinkGeometryText(
  nodeRows: NMNode[],
  fromNodeId: string,
  toNodeId: string,
) {
  return prettyGeometry(buildDefaultLineGeometry(nodeRows, fromNodeId, toNodeId));
}

export function buildToggleLinkPickResult(args: {
  currentEnabled: boolean;
  drawMode: LinkPickDrawMode;
  nodeRows: NMNode[];
}) {
  const nextEnabled = !args.currentEnabled;
  return {
    linkPickMode: nextEnabled,
    linkPickStep: 'from' as LinkPickStep,
    linkPathBendPoints: [] as Array<[number, number]>,
    resetFromNodeId: nextEnabled,
    resetToNodeId: nextEnabled,
    geometryText:
      args.drawMode === 'quick'
        ? buildStraightLinkGeometryText(args.nodeRows, '', '')
        : createEmptyLineGeometryText(),
    toastMessage: nextEnabled
      ? args.drawMode === 'quick'
        ? 'Quick mode: click source node, then destination node.'
        : 'Path mode: click source node, add bend points on map, then click destination node.'
      : null,
  };
}

export function buildSetLinkDrawModeResult(args: {
  mode: LinkPickDrawMode;
  linkPickMode: boolean;
  nodeRows: NMNode[];
}) {
  return {
    linkPickDrawMode: args.mode,
    linkPickStep: 'from' as LinkPickStep,
    linkPathBendPoints: [] as Array<[number, number]>,
    geometryText:
      args.mode === 'quick'
        ? buildStraightLinkGeometryText(args.nodeRows, '', '')
        : createEmptyLineGeometryText(),
    toastMessage: args.linkPickMode
      ? args.mode === 'quick'
        ? 'Quick mode active: click source then destination node.'
        : 'Path mode active: click source node, add bend points, then click destination node.'
      : null,
  };
}

export function buildConnectFromNodeResult(nodeId: string, nodeRows: NMNode[]) {
  const sourceNode = nodeRows.find((x) => x.id === nodeId);
  return {
    editingLinkId: null as string | null,
    showLinkModal: false,
    linkPickDrawMode: 'path' as LinkPickDrawMode,
    linkPickMode: true,
    linkPickStep: 'to' as LinkPickStep,
    linkPathBendPoints: [] as Array<[number, number]>,
    linkForm: {
      name: sourceNode ? `Link ${sourceNode.name}` : '',
      link_type: 'fiber',
      status: 'up',
      from_node_id: nodeId,
      to_node_id: '',
      priority: '100',
      capacity_mbps: '',
      utilization_pct: '',
      loss_db: '',
      latency_ms: '',
      geometryText: createEmptyLineGeometryText(),
    } satisfies NetworkMapLinkForm,
    toastMessage: 'Connect mode active: draw path on map, then click destination marker.',
  };
}

export function buildHandlePickedLinkNodeResult(args: {
  nodeId: string;
  linkPickMode: boolean;
  linkPickStep: LinkPickStep;
  linkPickDrawMode: LinkPickDrawMode;
  linkRows: NMLink[];
  nodeRows: NMNode[];
  linkForm: NetworkMapLinkForm;
  editingLinkId: string | null;
}) {
  if (!args.linkPickMode) return { kind: 'noop' as const };

  if (args.linkPickStep === 'from') {
    return {
      kind: 'picked-from' as const,
      linkForm: {
        ...args.linkForm,
        from_node_id: args.nodeId,
        to_node_id: '',
      },
      linkPathBendPoints: [] as Array<[number, number]>,
      linkPickStep: 'to' as LinkPickStep,
      toastMessage:
        args.linkPickDrawMode === 'quick'
          ? 'Source selected. Click destination node.'
          : 'Source selected. Click map to add bend points, then click destination node.',
    };
  }

  if (args.linkForm.from_node_id === args.nodeId) {
    return {
      kind: 'error' as const,
      toastMessage: 'Destination node must be different.',
    };
  }

  if (
    hasExistingLinkBetweenNodes(
      args.linkRows,
      args.linkForm.from_node_id,
      args.nodeId,
      args.editingLinkId,
    )
  ) {
    return {
      kind: 'error' as const,
      toastMessage: 'This node pair already has a link. Choose another destination node.',
    };
  }

  const nextForm = {
    ...args.linkForm,
    to_node_id: args.nodeId,
  };
  const fromName = args.nodeRows.find((x) => x.id === nextForm.from_node_id)?.name || 'Source';
  const toName = args.nodeRows.find((x) => x.id === nextForm.to_node_id)?.name || 'Destination';

  return {
    kind: 'picked-to' as const,
    linkForm: {
      ...nextForm,
      name: nextForm.name.trim() ? nextForm.name : `Link ${fromName} -> ${toName}`,
    },
    linkPickMode: false,
    linkPickStep: 'from' as LinkPickStep,
    showLinkModal: true,
    toastMessage: 'Endpoints selected from map.',
  };
}
