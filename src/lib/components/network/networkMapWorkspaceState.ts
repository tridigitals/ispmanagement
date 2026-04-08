export type NetworkMapWorkspaceCapabilities = {
  canManageTopology: boolean;
  canReadCustomers: boolean;
  canReadWorkOrders: boolean;
  canReadNetworkNoc: boolean;
  canReadRouterInventory: boolean;
};

export type NetworkMapWorkspaceMode = 'overview' | 'manage' | 'investigate';
export type NetworkMapInvestigationKind = 'service' | 'trace';

export type NetworkMapWorkspaceSelectedObjectKind =
  | 'node'
  | 'link'
  | 'zone'
  | 'router'
  | 'customer'
  | 'service';

export type NetworkMapWorkspaceSelectedObject = {
  kind: NetworkMapWorkspaceSelectedObjectKind;
  id: string;
  label: string;
  nodeType?: string;
  linkType?: string;
  zoneType?: string;
};

export type NetworkMapInvestigationState = {
  mode: NetworkMapInvestigationKind;
  rootObject: NetworkMapWorkspaceSelectedObject | null;
  selectedObject: NetworkMapWorkspaceSelectedObject | null;
  startedFrom: 'selection' | 'manual';
};

export type NetworkMapWorkspaceDefaults = {
  mode: NetworkMapWorkspaceMode;
  investigationKind: NetworkMapInvestigationKind;
  overviewAvailable: boolean;
  manageAvailable: boolean;
};

export type NetworkMapWorkspaceState = NetworkMapWorkspaceDefaults & {
  selectedObject: NetworkMapWorkspaceSelectedObject | null;
  investigationState: NetworkMapInvestigationState | null;
};

export type BuildSelectedMapObjectInput = {
  kind: NetworkMapWorkspaceSelectedObjectKind;
  id: string;
  label?: string | null;
  name?: string | null;
  nodeType?: string | null;
  linkType?: string | null;
  zoneType?: string | null;
};

export type BuildInvestigationStateInput = {
  mode: NetworkMapInvestigationKind;
  rootObject?: NetworkMapWorkspaceSelectedObject | null;
  selectedObject?: NetworkMapWorkspaceSelectedObject | null;
};

function isInvestigationPreferred(capabilities: NetworkMapWorkspaceCapabilities): boolean {
  return (
    capabilities.canReadCustomers ||
    capabilities.canReadWorkOrders ||
    capabilities.canReadNetworkNoc ||
    capabilities.canReadRouterInventory
  );
}

export function getNetworkMapDefaultInvestigationKind(
  capabilities: NetworkMapWorkspaceCapabilities,
): NetworkMapInvestigationKind {
  return capabilities.canReadNetworkNoc && !capabilities.canReadCustomers ? 'trace' : 'service';
}

export function getNetworkMapDefaultMode(
  capabilities: NetworkMapWorkspaceCapabilities,
): NetworkMapWorkspaceMode {
  if (capabilities.canManageTopology) return 'manage';
  if (isInvestigationPreferred(capabilities)) return 'investigate';
  return 'overview';
}

export function buildNetworkMapWorkspaceDefaults(
  capabilities: NetworkMapWorkspaceCapabilities,
): NetworkMapWorkspaceDefaults {
  return {
    mode: getNetworkMapDefaultMode(capabilities),
    investigationKind: getNetworkMapDefaultInvestigationKind(capabilities),
    overviewAvailable: true,
    manageAvailable: capabilities.canManageTopology,
  };
}

export function applyNetworkMapWorkspaceDefaults(
  state: NetworkMapWorkspaceState,
  defaults: NetworkMapWorkspaceDefaults,
): NetworkMapWorkspaceState {
  return {
    ...state,
    ...defaults,
    mode: state.selectedObject || state.investigationState ? state.mode : defaults.mode,
  };
}

export function buildSelectedMapObject(
  input: BuildSelectedMapObjectInput,
): NetworkMapWorkspaceSelectedObject {
  const label = String(input.label ?? input.name ?? input.id ?? '').trim() || input.id;
  const selectedObject: NetworkMapWorkspaceSelectedObject = {
    kind: input.kind,
    id: String(input.id || '').trim(),
    label,
  };

  if (input.nodeType) selectedObject.nodeType = String(input.nodeType).trim();
  if (input.linkType) selectedObject.linkType = String(input.linkType).trim();
  if (input.zoneType) selectedObject.zoneType = String(input.zoneType).trim();

  return selectedObject;
}

export function buildInvestigationState(
  input: BuildInvestigationStateInput,
): NetworkMapInvestigationState {
  const rootObject = input.rootObject ?? input.selectedObject ?? null;
  const selectedObject = input.selectedObject ?? rootObject;
  return {
    mode: input.mode,
    rootObject,
    selectedObject,
    startedFrom: selectedObject ? 'selection' : 'manual',
  };
}

export function createNetworkMapWorkspaceState(
  capabilities: NetworkMapWorkspaceCapabilities,
): NetworkMapWorkspaceState {
  return {
    ...buildNetworkMapWorkspaceDefaults(capabilities),
    selectedObject: null,
    investigationState: null,
  };
}

export function selectNetworkMapObject(
  state: NetworkMapWorkspaceState,
  selectedObject: NetworkMapWorkspaceSelectedObject | null,
): NetworkMapWorkspaceState {
  return {
    ...state,
    selectedObject: selectedObject ? buildSelectedMapObject(selectedObject) : null,
    investigationState: null,
  };
}

export function enterInvestigationMode(
  state: NetworkMapWorkspaceState,
  mode: NetworkMapInvestigationKind,
  rootObject?: NetworkMapWorkspaceSelectedObject | null,
): NetworkMapWorkspaceState {
  const nextRootObject = rootObject ?? state.selectedObject ?? null;
  return {
    ...state,
    mode: 'investigate',
    investigationState: buildInvestigationState({
      mode,
      rootObject: nextRootObject,
      selectedObject: nextRootObject,
    }),
  };
}

export function exitInvestigationMode(state: NetworkMapWorkspaceState): NetworkMapWorkspaceState {
  return {
    ...state,
    investigationState: null,
  };
}

export function clearWorkspaceSelection(
  state: NetworkMapWorkspaceState,
): NetworkMapWorkspaceState {
  return {
    ...state,
    selectedObject: null,
    investigationState: null,
  };
}
