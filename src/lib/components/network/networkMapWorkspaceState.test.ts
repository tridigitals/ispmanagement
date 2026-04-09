import { describe, expect, it } from 'vitest';

import {
  buildInvestigationState,
  applyNetworkMapWorkspaceDefaults,
  buildImpactedServiceList,
  buildNetworkMapWorkspaceDefaults,
  buildSelectedMapObject,
  clearWorkspaceSelection,
  createNetworkMapWorkspaceState,
  enterInvestigationMode,
  exitInvestigationMode,
  selectNetworkMapObject,
} from './networkMapWorkspaceState';

const technicianCapabilities = {
  canManageTopology: false,
  canReadCustomers: true,
  canReadWorkOrders: true,
  canReadNetworkNoc: false,
  canReadRouterInventory: true,
};

const nocCapabilities = {
  canManageTopology: false,
  canReadCustomers: false,
  canReadWorkOrders: false,
  canReadNetworkNoc: true,
  canReadRouterInventory: true,
};

const manageCapabilities = {
  canManageTopology: true,
  canReadCustomers: true,
  canReadWorkOrders: true,
  canReadNetworkNoc: true,
  canReadRouterInventory: true,
};

describe('network map workspace state', () => {
  it('uses service investigation defaults for technician-style access', () => {
    expect(buildNetworkMapWorkspaceDefaults(technicianCapabilities)).toMatchObject({
      mode: 'investigate',
      investigationKind: 'service',
      overviewAvailable: true,
      manageAvailable: false,
    });
  });

  it('uses trace investigation defaults for NOC-style access', () => {
    expect(buildNetworkMapWorkspaceDefaults(nocCapabilities)).toMatchObject({
      mode: 'investigate',
      investigationKind: 'trace',
      overviewAvailable: true,
      manageAvailable: false,
    });
  });

  it('keeps overview and manage availability on for topology editors', () => {
    expect(buildNetworkMapWorkspaceDefaults(manageCapabilities)).toMatchObject({
      mode: 'manage',
      overviewAvailable: true,
      manageAvailable: true,
    });
  });

  it('stores a selected node as the selectedObject', () => {
    const workspace = createNetworkMapWorkspaceState(technicianCapabilities);
    const selectedNode = buildSelectedMapObject({
      kind: 'node',
      id: 'node-17',
      label: 'Core POP 17',
      nodeType: 'router',
    });

    const next = selectNetworkMapObject(workspace, selectedNode);

    expect(next.selectedObject).toEqual(selectedNode);
    expect(next.investigationState).toBeNull();
  });

  it('builds an investigationState when trace mode starts from a selected node', () => {
    const workspace = {
      ...createNetworkMapWorkspaceState(nocCapabilities),
      mode: 'overview' as const,
    };
    const selectedNode = buildSelectedMapObject({
      kind: 'node',
      id: 'node-18',
      label: 'Backbone POP',
      nodeType: 'core',
    });

    const selected = selectNetworkMapObject(workspace, selectedNode);
    const next = enterInvestigationMode(selected, 'trace');

    expect(next.selectedObject).toEqual(selectedNode);
    expect(next.mode).toBe('investigate');
    expect(next.investigationState).toEqual(
      expect.objectContaining({
        mode: 'trace',
        rootObject: selectedNode,
        selectedObject: selectedNode,
      }),
    );
    expect(buildInvestigationState({ mode: 'trace', rootObject: selectedNode })).toEqual(
      expect.objectContaining({
        mode: 'trace',
        rootObject: selectedNode,
      }),
    );
  });

  it('applies workspace defaults without dropping an active selection', () => {
    const selectedNode = buildSelectedMapObject({
      kind: 'node',
      id: 'node-20',
      label: 'Spur Node',
    });
    const workspace = {
      ...createNetworkMapWorkspaceState(manageCapabilities),
      mode: 'overview' as const,
      selectedObject: selectedNode,
    };

    const next = applyNetworkMapWorkspaceDefaults(workspace, {
      mode: 'manage',
      investigationKind: 'service',
      overviewAvailable: true,
      manageAvailable: true,
    });

    expect(next.mode).toBe('overview');
    expect(next.selectedObject).toEqual(selectedNode);
    expect(next.manageAvailable).toBe(true);
  });

  it('clears both selection and investigation state together', () => {
    const workspace = createNetworkMapWorkspaceState(nocCapabilities);
    const selectedNode = buildSelectedMapObject({
      kind: 'node',
      id: 'node-19',
      label: 'Spur Node',
    });

    const investigating = enterInvestigationMode(
      selectNetworkMapObject(workspace, selectedNode),
      'trace',
    );
    const cleared = clearWorkspaceSelection(investigating);

    expect(cleared.selectedObject).toBeNull();
    expect(cleared.investigationState).toBeNull();
  });

  it('returns to a stable non-investigation mode when investigation ends', () => {
    const workspace = createNetworkMapWorkspaceState(manageCapabilities);
    const selectedNode = buildSelectedMapObject({
      kind: 'node',
      id: 'node-21',
      label: 'Managed Core Node',
    });

    const investigating = enterInvestigationMode(
      selectNetworkMapObject(workspace, selectedNode),
      'trace',
    );
    const exited = exitInvestigationMode(investigating);

    expect(exited.mode).toBe('manage');
    expect(exited.investigationState).toBeNull();
    expect(exited.selectedObject).toEqual(selectedNode);
  });

  it('keeps the investigation root while allowing focus to move deeper into the trace', () => {
    const workspace = createNetworkMapWorkspaceState(nocCapabilities);
    const rootNode = buildSelectedMapObject({
      kind: 'node',
      id: 'node-root',
      label: 'Core POP',
    });
    const tracedLink = buildSelectedMapObject({
      kind: 'link',
      id: 'link-mid',
      label: 'Backbone Fiber',
      linkType: 'fiber',
    });

    const selected = selectNetworkMapObject(workspace, rootNode);
    const investigating = enterInvestigationMode(selected, 'trace', rootNode);
    const deeperTrace = buildInvestigationState({
      mode: 'trace',
      rootObject: investigating.investigationState?.rootObject,
      selectedObject: tracedLink,
    });

    expect(deeperTrace.rootObject).toEqual(rootNode);
    expect(deeperTrace.selectedObject).toEqual(tracedLink);
    expect(deeperTrace.startedFrom).toBe('selection');
  });

  it('builds a safe impacted-service list from sparse service data', () => {
    const services = buildImpactedServiceList({
      rootObject: buildSelectedMapObject({
        kind: 'node',
        id: 'node-service',
        label: 'Access POP',
      }),
      services: [
        { id: 'svc-1', name: 'Office A', status: 'active' },
        { id: 'svc-2', customerName: 'PT Maju', status: 'degraded' },
        { id: '', label: 'Fallback Label' },
      ],
    });

    expect(services).toEqual([
      expect.objectContaining({ id: 'svc-1', label: 'Office A', status: 'active' }),
      expect.objectContaining({ id: 'svc-2', label: 'PT Maju', status: 'degraded' }),
      expect.objectContaining({
        id: 'node-service-service-3',
        label: 'Fallback Label',
        status: 'unknown',
      }),
    ]);
  });
});
