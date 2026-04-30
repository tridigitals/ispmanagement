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

export const loadWallboardInsightsModules = createCachedLoader(async () => {
  const [{ default: InsightsControlsComponent }, { default: InsightsSummaryComponent }] =
    await Promise.all([
      import('$lib/components/network/WallboardInsightsControls.svelte'),
      import('$lib/components/network/WallboardInsightsSummary.svelte'),
    ]);

  return {
    InsightsControlsComponent,
    InsightsSummaryComponent,
  };
});

export const loadWallboardAlertsPanel = createCachedLoader(async () => {
  const { default: AlertsPanelComponent } = await import(
    '$lib/components/network/WallboardAlertsPanel.svelte'
  );

  return {
    AlertsPanelComponent,
  };
});

export const loadWallboardInsightsShell = createCachedLoader(async () => {
  const { default: InsightsShellComponent } = await import('./WallboardInsightsShell.svelte');

  return {
    InsightsShellComponent,
  };
});

export const loadWallboardDialogs = createCachedLoader(async () => {
  const [
    { default: SlotPickerComponent },
    { default: FullDialogComponent },
    { default: ThresholdDialogComponent },
  ] = await Promise.all([
    import('$lib/components/network/WallboardSlotPicker.svelte'),
    import('$lib/components/network/WallboardFullDialog.svelte'),
    import('$lib/components/network/WallboardThresholdDialog.svelte'),
  ]);

  return {
    SlotPickerComponent,
    FullDialogComponent,
    ThresholdDialogComponent,
  };
});

export const loadWallboardExportModule = createCachedLoader(async () => {
  const module = await import('$lib/utils/tabularExport');

  return {
    exportCsvRows: module.exportCsvRows,
  };
});
