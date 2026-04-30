type DeferredComponent = any;
type AsyncModuleLoader<T> = () => Promise<T>;

function createCachedLoader<T>(loader: AsyncModuleLoader<T>): AsyncModuleLoader<T> {
  let cached: Promise<T> | null = null;

  return () => {
    if (!cached) {
      cached = loader();
    }
    return cached;
  };
}

export type AlertsPageModules = {
  AlertsIncidentsSwitchComponent: DeferredComponent;
  NetworkFilterPanelComponent: DeferredComponent;
  NetworkPageHeaderComponent: DeferredComponent;
  RowActionButtonsComponent: DeferredComponent;
};

export const loadAlertsPageModules = createCachedLoader<AlertsPageModules>(async () => {
  const [
    { default: AlertsIncidentsSwitchComponent },
    { default: NetworkFilterPanelComponent },
    { default: NetworkPageHeaderComponent },
    { default: RowActionButtonsComponent },
  ] = await Promise.all([
    import('$lib/components/network/AlertsIncidentsSwitch.svelte'),
    import('$lib/components/network/NetworkFilterPanel.svelte'),
    import('$lib/components/network/NetworkPageHeader.svelte'),
    import('$lib/components/network/RowActionButtons.svelte'),
  ]);

  return {
    AlertsIncidentsSwitchComponent,
    NetworkFilterPanelComponent,
    NetworkPageHeaderComponent,
    RowActionButtonsComponent,
  };
});
