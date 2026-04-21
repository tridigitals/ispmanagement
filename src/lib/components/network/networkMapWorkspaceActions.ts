import {
  buildInvestigationState,
  buildSelectedMapObject,
  type NetworkMapInvestigationKind,
  type NetworkMapWorkspaceSelectedObject,
  type NetworkMapWorkspaceState,
} from './networkMapWorkspaceState';

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

export function clearWorkspaceSelection(state: NetworkMapWorkspaceState): NetworkMapWorkspaceState {
  return {
    ...state,
    selectedObject: null,
    investigationState: null,
  };
}
