import type { Component } from 'svelte';

type DeferredComponent = Component<any>;

export type NetworkMapChromeModules = {
  OverviewComponent: DeferredComponent;
  FloatingControlsComponent: DeferredComponent;
};

export type NetworkMapDialogModules = {
  NodePanelComponent: DeferredComponent;
  LinkModalComponent: DeferredComponent;
  ZoneModalComponent: DeferredComponent;
  ConfirmDialogComponent: DeferredComponent;
};

export type NetworkMapPopupModule = typeof import('./networkMapPopups');
export type NetworkMapInteractionModule = typeof import('./networkMapInteractionRuntime');

let chromeModulesPromise: Promise<NetworkMapChromeModules> | null = null;
let dialogModulesPromise: Promise<NetworkMapDialogModules> | null = null;
let popupModulePromise: Promise<NetworkMapPopupModule> | null = null;
let interactionModulePromise: Promise<NetworkMapInteractionModule> | null = null;

export function loadNetworkMapChromeModules(): Promise<NetworkMapChromeModules> {
  if (!chromeModulesPromise) {
    chromeModulesPromise = Promise.all([
      import('$lib/components/network/NetworkMapOverview.svelte'),
      import('$lib/components/network/NetworkMapFloatingControls.svelte'),
    ]).then(([overview, floatingControls]) => ({
      OverviewComponent: overview.default,
      FloatingControlsComponent: floatingControls.default,
    }));
  }

  return chromeModulesPromise!;
}

export function loadNetworkMapDialogModules(): Promise<NetworkMapDialogModules> {
  if (!dialogModulesPromise) {
    dialogModulesPromise = Promise.all([
      import('$lib/components/network/NetworkMapNodePanel.svelte'),
      import('$lib/components/network/NetworkMapLinkModal.svelte'),
      import('$lib/components/network/NetworkMapZoneModal.svelte'),
      import('$lib/components/ui/ConfirmDialog.svelte'),
    ]).then(([nodePanel, linkModal, zoneModal, confirmDialog]) => ({
      NodePanelComponent: nodePanel.default,
      LinkModalComponent: linkModal.default,
      ZoneModalComponent: zoneModal.default,
      ConfirmDialogComponent: confirmDialog.default,
    }));
  }

  return dialogModulesPromise!;
}

export function loadNetworkMapPopupModule(): Promise<NetworkMapPopupModule> {
  if (!popupModulePromise) {
    popupModulePromise = import('./networkMapPopups');
  }

  return popupModulePromise!;
}

export function loadNetworkMapInteractionModule(): Promise<NetworkMapInteractionModule> {
  if (!interactionModulePromise) {
    interactionModulePromise = import('./networkMapInteractionRuntime');
  }

  return interactionModulePromise!;
}
