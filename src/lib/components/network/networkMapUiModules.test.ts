import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  overview: { name: 'overview-component' },
  floatingControls: { name: 'floating-controls-component' },
  nodePanel: { name: 'node-panel-component' },
  linkModal: { name: 'link-modal-component' },
  zoneModal: { name: 'zone-modal-component' },
  confirmDialog: { name: 'confirm-dialog-component' },
  interactionModule: {
    buildBaseMapStyle: vi.fn(),
    registerMapSourcesAndLayers: vi.fn(),
    ensureNodeTypeIconsRegistered: vi.fn(),
    registerPrimaryLayerClicks: vi.fn(),
    registerInteractiveLayerHover: vi.fn(),
    expandCustomerCluster: vi.fn(),
    handleCanvasMapClick: vi.fn(),
    fitMapToMarkers: vi.fn(),
    buildSelectionFeatureCollections: vi.fn(),
  },
  workspaceModule: {
    selectNetworkMapObject: vi.fn(),
    enterInvestigationMode: vi.fn(),
    clearWorkspaceSelection: vi.fn(),
  },
  popupModule: {
    openNodePopup: vi.fn(),
    openLinkPopup: vi.fn(),
    openRouterPopup: vi.fn(),
  },
}));

vi.mock('$lib/components/network/NetworkMapOverview.svelte', () => ({
  default: sentinels.overview,
}));

vi.mock('$lib/components/network/NetworkMapFloatingControls.svelte', () => ({
  default: sentinels.floatingControls,
}));

vi.mock('$lib/components/network/NetworkMapNodePanel.svelte', () => ({
  default: sentinels.nodePanel,
}));

vi.mock('$lib/components/network/NetworkMapLinkModal.svelte', () => ({
  default: sentinels.linkModal,
}));

vi.mock('$lib/components/network/NetworkMapZoneModal.svelte', () => ({
  default: sentinels.zoneModal,
}));

vi.mock('$lib/components/ui/ConfirmDialog.svelte', () => ({
  default: sentinels.confirmDialog,
}));

vi.mock('$lib/components/network/networkMapInteractionRuntime', () => sentinels.interactionModule);
vi.mock('$lib/components/network/networkMapWorkspaceActions', () => sentinels.workspaceModule);

vi.mock('$lib/components/network/networkMapPopups', () => sentinels.popupModule);

import {
  loadNetworkMapChromeModules,
  loadNetworkMapDialogModules,
  loadNetworkMapInteractionModule,
  loadNetworkMapPopupModule,
  loadNetworkMapWorkspaceModule,
} from './networkMapUiModules';

describe('network map ui modules', () => {
  it('loads and caches the chrome modules used around the map canvas', async () => {
    const first = await loadNetworkMapChromeModules();
    const second = await loadNetworkMapChromeModules();

    expect(first).toEqual({
      OverviewComponent: sentinels.overview,
      FloatingControlsComponent: sentinels.floatingControls,
    });
    expect(second).toBe(first);
  });

  it('loads and caches the on-demand modal and panel modules', async () => {
    const first = await loadNetworkMapDialogModules();
    const second = await loadNetworkMapDialogModules();

    expect(first).toEqual({
      NodePanelComponent: sentinels.nodePanel,
      LinkModalComponent: sentinels.linkModal,
      ZoneModalComponent: sentinels.zoneModal,
      ConfirmDialogComponent: sentinels.confirmDialog,
    });
    expect(second).toBe(first);
  });

  it('loads and caches the popup module for interactive map clicks', async () => {
    const first = await loadNetworkMapPopupModule();
    const second = await loadNetworkMapPopupModule();

    expect(first.openNodePopup).toBe(sentinels.popupModule.openNodePopup);
    expect(first.openLinkPopup).toBe(sentinels.popupModule.openLinkPopup);
    expect(first.openRouterPopup).toBe(sentinels.popupModule.openRouterPopup);
    expect(second).toBe(first);
  });

  it('loads and caches the map interaction runtime module', async () => {
    const first = await loadNetworkMapInteractionModule();
    const second = await loadNetworkMapInteractionModule();

    expect(first.buildBaseMapStyle).toBe(sentinels.interactionModule.buildBaseMapStyle);
    expect(first.registerMapSourcesAndLayers).toBe(
      sentinels.interactionModule.registerMapSourcesAndLayers,
    );
    expect(first.handleCanvasMapClick).toBe(sentinels.interactionModule.handleCanvasMapClick);
    expect(first.buildSelectionFeatureCollections).toBe(
      sentinels.interactionModule.buildSelectionFeatureCollections,
    );
    expect(second).toBe(first);
  });

  it('loads and caches the workspace action module', async () => {
    const first = await loadNetworkMapWorkspaceModule();
    const second = await loadNetworkMapWorkspaceModule();

    expect(first.selectNetworkMapObject).toBe(sentinels.workspaceModule.selectNetworkMapObject);
    expect(first.enterInvestigationMode).toBe(sentinels.workspaceModule.enterInvestigationMode);
    expect(first.clearWorkspaceSelection).toBe(sentinels.workspaceModule.clearWorkspaceSelection);
    expect(second).toBe(first);
  });
});
